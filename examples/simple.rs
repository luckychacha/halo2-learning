use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use halo2_proofs::{dev::MockProver, pasta::Fp};

#[derive(Debug, Clone)]
struct CircuitConfig {
    advice: [Column<Advice>; 2],
    instance: Column<Instance>,
    selector: Selector,
}

#[derive(Default)]
struct MyCircuit<F: Field> {
    c: F,
    a: Value<F>,
    b: Value<F>,
}

#[derive(Clone)]
struct Number<F: Field>(AssignedCell<F, F>);

fn load_private<F: Field>(
    config: &CircuitConfig,
    mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    value: Value<F>,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "load private",
        |mut region| {
            region
                .assign_advice(|| "load private", config.advice[0], 0, || value)
                .map(Number)
        },
    )
}

fn load_constant<F: Field>(
    config: &CircuitConfig,
    mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    constant: F,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "load constant",
        |mut region| {
            region
                .assign_advice_from_constant(|| "load constant", config.advice[0], 0, constant)
                .map(Number)
        },
    )
}

fn mul<F: Field>(
    config: &CircuitConfig,
    mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    a: Number<F>,
    b: Number<F>,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "mul",
        |mut region| {
            config.selector.enable(&mut region, 0)?;
            a.0.copy_advice(|| "lhs", &mut region, config.advice[0], 0)?;
            b.0.copy_advice(|| "rhs", &mut region, config.advice[1], 0)?;

            let value = a.0.value().copied() * b.0.value().copied();

            region
                .assign_advice(|| "out = lhs * rhs", config.advice[0], 1, || value)
                .map(Number)
        },
    )
}

impl<F: Field> Circuit<F> for MyCircuit<F> {
    type Config = CircuitConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        let advice = [meta.advice_column(), meta.advice_column()];

        let instance = meta.instance_column();

        let selector = meta.selector();
        let constant = meta.fixed_column();

        meta.enable_equality(instance);
        meta.enable_constant(constant);

        for c in &advice {
            meta.enable_equality(*c);
        }

        let gate = meta.create_gate("mul_gate", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());

            let mul_selector = meta.query_selector(selector);

            vec![mul_selector * (lhs * rhs - out)]
        });

        CircuitConfig {
            advice,
            instance,
            selector,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let a = load_private(&config, layouter.namespace(|| "load a"), self.a)?;
        let b = load_private(&config, layouter.namespace(|| "load b"), self.b)?;
        let c = load_constant(&config, layouter.namespace(|| "load c"), self.c)?;

        let ab = mul(&config, layouter.namespace(|| "a * b"), a, b)?;
        let absq = mul(&config, layouter.namespace(|| "ab * ab"), ab.clone(), ab)?;
        let out = mul(&config, layouter.namespace(|| "absq * c"), absq, c)?;

        // expose public
        layouter
            .namespace(|| "expose out")
            .constrain_instance(out.0.cell(), config.instance, 0)
    }
}

fn main() {
    test_1();
    #[cfg(feature = "dev-graph")]
    plot_1_circuit();
}

fn test_1() {
    // ANCHOR: test-circuit
    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    let k = 5;

    // Prepare the private and public inputs to the circuit!
    let c = Fp::from(1);
    let a = Fp::from(2);
    let b = Fp::from(3);
    let out = c * a.square() * b.square();
    println!("out=:{:?}", out);

    // Instantiate the circuit with the private inputs.
    let circuit = MyCircuit {
        c,
        a: Value::known(a),
        b: Value::known(b),
    };

    // Arrange the public input. We expose the multiplication result in row 0
    // of the instance column, so we position it there in our public inputs.
    let mut public_inputs = vec![out];

    // Given the correct public input, our circuit will verify.
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // If we try some other public input, the proof will fail!
    public_inputs[0] += Fp::one();
    let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    assert!(prover.verify().is_err());
    println!("\n\n\n!!!!!OHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH!!!!!\n     simple example success !\n!!!!!OHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH!!!!!\n\n\n")
    // ANCHOR_END: test-circuit
}

#[cfg(feature = "dev-graph")]
fn plot_1_circuit() {
    println!("Plot");
    // Instantiate the circuit with the private inputs.
    let circuit = MyCircuit::<Fp>::default();
    // Create the area you want to draw on.
    // Use SVGBackend if you want to render to .svg instead.
    use plotters::prelude::*;
    let root = BitMapBackend::new("chap_1_simple.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root
        .titled("Simple Circuit without chip", ("sans-serif", 60))
        .unwrap();
    halo2_proofs::dev::CircuitLayout::default()
        // You can optionally render only a section of the circuit.
        // .view_width(0..2)
        // .view_height(0..16)
        // You can hide labels, which can be useful with smaller areas.
        .show_labels(true)
        // Render the circuit onto your area!
        // The first argument is the size parameter for the circuit.
        .render(5, &circuit, &root)
        .unwrap();
}
