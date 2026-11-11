//! Satelliti di Aurora: orbite SGP4, passaggi, azimut ed elevazione, Doppler sulla radio e tracking con il rotore.
//!
//! - [`orbit`]: propagazione (libreria `sgp4`, MIT), vista dall'osservatore, passaggi, Sole e Luna;
//! - [`catalog`]: i satelliti radioamatoriali con i loro transponder (lineari, invertenti, FM);
//! - [`tle`]: i dati delle orbite da CelesTrak;
//! - [`service`]: il tracking ogni secondo con la correzione Doppler della radio e il rotore.

pub mod catalog;
pub mod orbit;
pub mod service;
pub mod tle;

pub use catalog::{SatInfo, Transponder};
pub use orbit::{Look, Observer, Pass, Sat, maidenhead_to_latlon};
pub use service::{DopplerPlan, Now, Position, SatRadio, SatRotator, SatService, SatSettings};
