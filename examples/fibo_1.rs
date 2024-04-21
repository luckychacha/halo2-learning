use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner},
    dev::MockProver,
    pasta::Fp,
    plonk::{Advice, Circuit, Column, Error, Instance, Selector},
    poly::Rotation,
};

struct ACell<F: Field>(AssignedCell<F, F>);

#[derive(Clone, Debug)]
struct FiboConfig {
    instance: Column<Instance>,
    advice: Column<Advice>,
    selector: Selector,
}

#[derive(Clone, Debug)]
struct FiboChip<F: Field> {
    config: FiboConfig,
    _phantom: PhantomData<F>,
}

impl<F: Field> FiboChip<F> {
    fn construct(config: FiboConfig) -> Self {
        FiboChip {
            config,
            _phantom: PhantomData,
        }
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> FiboConfig {
        let advice = meta.advice_column();
        let instance = meta.instance_column();
        let selector = meta.selector();
        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate("fibo gate", |meta| {
            let curr = meta.query_advice(advice, Rotation::cur());
            let next = meta.query_advice(advice, Rotation::next());
            let third = meta.query_advice(advice, Rotation(2));

            let selector = meta.query_selector(selector);

            vec![selector * (curr + next - third)]
        });

        FiboConfig {
            advice,
            instance,
            selector,
        }
    }

    fn assign(
        &self,
        nrow: usize,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "fibo",
            |mut region| {
                let advice = self.config.advice;
                let instance = self.config.instance;

                let selector = self.config.selector;
                selector.enable(&mut region, 0)?;
                selector.enable(&mut region, 1)?;

                let mut f_pre = region
                    .assign_advice_from_instance(|| "f0", instance, 0, advice, 0)
                    .map(ACell)?;
                let mut f_curr = region
                    .assign_advice_from_instance(|| "f1", instance, 1, advice, 1)
                    .map(ACell)?;

                for i in 2..nrow {
                    if i < nrow - 2 {
                        selector.enable(&mut region, i)?;
                    }
                    let value = f_pre.0.value().copied() + f_curr.0.value();

                    let f_next = region
                        .assign_advice(|| "fn", advice, i, || value)
                        .map(ACell)?;

                    f_pre = f_curr;
                    f_curr = f_next;
                }
                Ok(f_curr)
            },
        )
    }
}

#[derive(Clone, Debug, Default)]
struct FiboCircuit<F: Field> {
    nrow: usize,
    _phantom: PhantomData<F>,
}

impl<F: Field> Circuit<F> for FiboCircuit<F> {
    type Config = FiboConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        FiboChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FiboChip::construct(config);
        let out = FiboChip::assign(&chip, self.nrow, layouter.namespace(|| "fibo table"))?;

        layouter
            .namespace(|| "out")
            .constrain_instance(out.0.cell(), chip.config.instance, 2)
    }
}

fn main() {
    let f0 = Fp::from(1);
    let f1 = Fp::from(1);
    let out = Fp::from(55);
    let circuit = FiboCircuit {
        nrow: 10,
        _phantom: PhantomData,
    };

    let k = 4;
    let public_inputs = vec![f0, f1, out];
    println!("out: {:?}", out);
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
}
