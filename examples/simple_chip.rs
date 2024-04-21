use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, SimpleFloorPlanner, Value},
    dev::MockProver,
    pasta::Fp,
    plonk::{Advice, Circuit, Column, Constraints, Instance, Selector},
    poly::Rotation,
};

#[derive(Default)]
struct MyCircuit<F: Field> {
    a: Value<F>,
    b: Value<F>,
    c: F,
}

#[derive(Clone, Debug)]
struct CircuitConfig {
    advice: [Column<Advice>; 2],
    instance: Column<Instance>,
    s_mul: Selector,
    s_add: Selector,
    s_cube: Selector,
}

#[derive(Debug, Clone)]
struct MyChip<F: Field> {
    config: CircuitConfig,
    _phantom: PhantomData<F>,
}

impl<F: Field> MyChip<F> {
    fn construct(config: CircuitConfig) -> Self {
        MyChip {
            config,
            _phantom: PhantomData,
        }
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> CircuitConfig {
        let advice = [meta.advice_column(), meta.advice_column()];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();

        let s_mul = meta.selector();
        let s_add = meta.selector();
        let s_cube = meta.selector();

        meta.enable_equality(instance);
        meta.enable_constant(constant);
        for c in advice {
            meta.enable_equality(c);
        }

        meta.create_gate("s_mul", |meta| {
            let a = meta.query_advice(advice[0], Rotation::cur());
            let b = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());
            let s_mul = meta.query_selector(s_mul);
            Constraints::with_selector(s_mul, vec![a * b - out])
        });

        meta.create_gate("s_add", |meta| {
            let a = meta.query_advice(advice[0], Rotation::cur());
            let b = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());
            let s_add = meta.query_selector(s_add);

            Constraints::with_selector(s_add, vec![a + b - out])
        });
        meta.create_gate("s_cube", |meta| {
            let a = meta.query_advice(advice[0], Rotation::cur());
            let out = meta.query_advice(advice[1], Rotation::cur());
            let s_cube = meta.query_selector(s_cube);

            Constraints::with_selector(s_cube, vec![a.clone() * a.clone() * a.clone() - out])
        });

        CircuitConfig {
            advice,
            instance,
            s_mul,
            s_add,
            s_cube,
        }
    }

    fn assign(
        &self,
        a: Value<F>,
        b: Value<F>,
        c: F,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<Number<F>, halo2_proofs::plonk::Error> {
        let config = &self.config;
        let cells = layouter
            .assign_region(
                || "load private",
                |mut region| {
                    let a_cell = region
                        .assign_advice(|| "load a", config.advice[0], 0, || a)
                        .map(Number)?;
                    let b_cell = region
                        .assign_advice(|| "load b", config.advice[0], 1, || b)
                        .map(Number)?;
                    let c_cell = region
                        .assign_advice_from_constant(|| "load c", config.advice[0], 2, c)
                        .map(Number)?;
                    Ok((a_cell, b_cell, c_cell))
                },
            )
            .unwrap();

        layouter.assign_region(
            || "load witness",
            |mut region| {
                let (a, b, c) = &cells;
                let mut offset = 0;
                // load a, b 0
                config.s_mul.enable(&mut region, offset)?;
                let a =
                    a.0.copy_advice(|| "lhs", &mut region, config.advice[0], offset)
                        .map(Number)?;
                let b =
                    b.0.copy_advice(|| "rhs", &mut region, config.advice[1], offset)
                        .map(Number)?;

                // fill ab, ab 1
                offset += 1;
                config.s_mul.enable(&mut region, offset)?;
                let value = a.0.value().copied() * b.0.value();
                let ab_0 = region
                    .assign_advice(|| "ab lhs", config.advice[0], offset, || value)
                    .map(Number)?;
                let ab_1 = ab_0
                    .0
                    .copy_advice(|| "ab rhs", &mut region, config.advice[1], offset)
                    .map(Number)?;

                // fill absq, c 2
                offset += 1;
                config.s_mul.enable(&mut region, offset)?;
                let value = ab_0.0.value().cloned() * ab_1.0.value().copied();
                let absq = region
                    .assign_advice(|| "absq", config.advice[0], offset, || value)
                    .map(Number)?;
                let c =
                    c.0.copy_advice(|| "rhs c", &mut region, config.advice[1], offset)
                        .map(Number)?;

                // fill c, d 3
                offset += 1;
                config.s_add.enable(&mut region, offset)?;
                let value = absq.0.value().copied() * c.0.value().copied();
                let d = region
                    .assign_advice(|| "d", config.advice[0], offset, || value)
                    .map(Number)?;
                let c =
                    c.0.copy_advice(|| "load c", &mut region, config.advice[1], offset)
                        .map(Number)?;

                // fill e and out
                offset += 1;
                let value = c.0.value().copied() + d.0.value().copied();
                let e = region
                    .assign_advice(|| "e", config.advice[0], offset, || value)
                    .map(Number)?;

                config.s_cube.enable(&mut region, offset)?;
                let value = e.0.value().copied() * e.0.value().copied() * e.0.value().copied();
                region
                    .assign_advice(|| "out", config.advice[1], offset, || value)
                    .map(Number)
            },
        )
    }

    fn expose_out(
        &self,
        out: Number<F>,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
        row: usize,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        layouter.constrain_instance(out.0.cell(), self.config.instance, row)
    }
}

struct Number<F: Field>(AssignedCell<F, F>);

impl<F: Field> Circuit<F> for MyCircuit<F> {
    type Config = CircuitConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        MyChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let chip = MyChip::construct(config);
        let out = chip.assign(self.a, self.b, self.c, layouter.namespace(|| "simple chip"))?;
        chip.expose_out(out, layouter, 0)
    }
}

fn main() {
    simple_chip();
}

fn simple_chip() {
    let a = Fp::from(2);
    let b = Fp::from(2);
    let c = Fp::from(3);
    let e = c * a.square() * b.square() + c;
    println!("e=:{:?}", e);

    let out = e.cube();
    println!("out=:{:?}", out);

    let my_circuit = MyCircuit {
        a: Value::known(a),
        b: Value::known(b),
        c,
    };

    let k = 5;

    let mut public_inputs = vec![out];
    let prover = MockProver::run(k, &my_circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    public_inputs[0] += Fp::one();
    let prover = MockProver::run(k, &my_circuit, vec![public_inputs]).unwrap();
    assert!(prover.verify().is_err());
    println!("Simple Chip Success");
}
