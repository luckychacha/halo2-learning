// use std::marker::PhantomData;

// use halo2_proofs::{
//     arithmetic::Field,
//     circuit::{AssignedCell, Layouter, SimpleFloorPlanner},
//     dev::MockProver,
//     pairing::{
//         bls12_381::{Fr, G1Affine, G2Affine, G1, G2},
//         group::Group,
//     },
//     plonk::{Advice, Circuit, Column, Error, Instance, Selector},
//     poly::Rotation,
// };
// use rand_core::OsRng;

// #[derive(Clone, Debug)]
// struct BLSConfig {
//     instance: Column<Instance>,
//     advice: [Column<Advice>; 2],
//     selector: Selector,
// }

// #[derive(Clone, Debug)]
// struct BLSChip<F: Field> {
//     config: BLSConfig,
//     _phantom: PhantomData<F>,
// }

// struct ACell<F: Field>(AssignedCell<F, F>);

// impl<F: Field> BLSChip<F> {
//     fn construct(config: BLSConfig) -> Self {
//         BLSChip {
//             config,
//             _phantom: PhantomData,
//         }
//     }

//     fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> BLSConfig {
//         let advice = [meta.advice_column(), meta.advice_column()];
//         let instance = meta.instance_column();
//         let selector = meta.selector();
//         for c in advice {
//             meta.enable_equality(c);
//         }
//         meta.enable_equality(instance);

//         meta.create_gate("BLS gate", |meta| {
//             let pub_key = meta.query_advice(advice[0], Rotation::cur());
//             let msg_hash = meta.query_advice(advice[1], Rotation::cur());
//             let g1 = meta.query_advice(advice[0], Rotation::next());
//             let sig = meta.query_advice(advice[1], Rotation::next());

//             let selector = meta.query_selector(selector);

//             vec![selector * (pairing(pub_key, msg_hash) - pairing(g1, sig))]
//         });

//         BLSConfig {
//             advice,
//             instance,
//             selector,
//         }
//     }

//     fn assign(
//         &self,
//         g1: G1Affine,
//         signature: G2Affine,
//         pubkey: G1Affine,
//         msg_hash: G2Affine,
//         mut layouter: impl halo2_proofs::circuit::Layouter<F>,
//     ) -> Result<(), Error> {
//         let config = &self.config;

//         let cells = layouter
//             .assign_region(
//                 || "load private",
//                 |mut region| {
//                     let g1 = region.assign_advice(|| "load g1", config.advice[0], 0, || g1)?;
//                     let signature =
//                         region.assign_advice(|| "load signature", config.advice[0], 1, || signature)?;
//                     let pubkey =
//                         region.assign_advice(|| "load pubkey", config.advice[0], 2, || pubkey)?;
//                     let msg_hash =
//                         region.assign_advice(|| "load msg_hash", config.advice[0], 3, || msg_hash)?;
//                     Ok((g1, signature, pubkey, msg_hash))
//                 },
//             )
//             .unwrap();

//         Ok(())
//     }
// }

// #[derive(Clone, Debug, Default)]
// struct BLSCircuit<F: Field> {
//     g1: G1Affine,
//     signature: G2Affine,
//     pubkey: G1Affine,
//     msg_hash: G2Affine,
//     _phantom: PhantomData<F>,
// }

// impl<F: Field> Circuit<F> for BLSCircuit<F> {
//     type Config = BLSConfig;

//     type FloorPlanner = SimpleFloorPlanner;

//     fn without_witnesses(&self) -> Self {
//         Self::default()
//     }

//     fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
//         BLSChip::configure(meta)
//     }

//     fn synthesize(
//         &self,
//         config: Self::Config,
//         mut layouter: impl halo2_proofs::circuit::Layouter<F>,
//     ) -> Result<(), Error> {
//         let chip = BLSChip::construct(config);
//         let out = BLSChip::assign(
//             &chip,
//             self.g1,
//             self.msg_hash,
//             self.pubkey,
//             self.signature,
//             layouter.namespace(|| "BLS"),
//         )?;

//         Ok(())
//     }
// }

// fn main() {
//     let msg_hash = G2Affine::from(G2::random(&mut OsRng));
//     let sk = Fr::random(OsRng);
//     let signature = G2Affine::from(msg_hash * sk);
//     let pubkey = G1Affine::from(G1Affine::generator() * sk);
//     let g1 = G1::generator();

//     let circuit = BLSCircuit {
//         g1: g1.into(),
//         signature,
//         pubkey,
//         msg_hash,
//         _phantom: PhantomData,
//     };

//     let k = 10;

//     let prover = MockProver::run(k, &circuit, vec![]).unwrap();
// }
