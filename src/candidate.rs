//! Position solving candidate

use gnss::prelude::SV;
use hifitime::Unit;
use log::debug;
use nyx_space::cosmic::SPEED_OF_LIGHT;

use crate::prelude::{Config, Duration, Epoch, InterpolationResult};
use crate::{Error, Vector3D};

/// Pseudo Range observation on a specific carrier frequency
#[derive(Debug, Default, Clone)]
pub struct PseudoRange {
    /// Pseudo Range raw value [m]
    pub value: f64,
    /// Carrier frequency [Hz]
    pub frequency: f64,
}

/// Position solving candidate
#[derive(Debug, Clone)]
pub struct Candidate {
    /// SV
    pub sv: SV,
    /// Signal sampling Epoch
    pub t: Epoch,
    /// state that needs to be resolved
    pub state: Option<InterpolationResult>,
    // SV group delay
    pub(crate) tgd: Option<Duration>,
    // SV clock state (compared to GNSS timescale)
    pub(crate) clock_state: Vector3D,
    // SV clock correction
    pub(crate) clock_corr: Duration,
    // SNR at sampling instant.
    pub(crate) snr: Option<f64>,
    /// Pseudo range observations at "t"
    pub(crate) pseudo_range: Vec<PseudoRange>,
}

impl Candidate {
    /// Creates a new candidate, to inject in the solver pool.
    /// SV : satellite vehicle.
    /// t: Epoch at which the signals were sampled.
    /// clock_state: SV clock state.
    /// clock_corr: SV clock correction to apply.
    /// snr: SNR at sampling instant, expressed in dB.
    /// Ideally, you should determine the worst SNR on all considered carriers.
    /// pseudo_range: PR observations on as many carriers as you want.
    pub fn new(
        sv: SV,
        t: Epoch,
        clock_state: Vector3D,
        clock_corr: Duration,
        snr: Option<f64>,
        pseudo_range: Vec<PseudoRange>,
    ) -> Result<Self, Error> {
        if pseudo_range.len() == 0 {
            Err(Error::NeedsAtLeastOnePseudoRange)
        } else {
            Ok(Self {
                sv,
                t,
                clock_state,
                clock_corr,
                snr,
                pseudo_range,
                tgd: None,
                state: None,
            })
        }
    }
    /*
     * Returns one pseudo range observation [m], disregarding its frequency.
     * Infaillible, because we don't allow to build Self without at least
     * 1 PR observation
     */
    pub(crate) fn pseudo_range(&self) -> &PseudoRange {
        self.pseudo_range
            .iter()
            // .map(|pr| pr.value)
            .reduce(|k, _| k)
            .unwrap()
    }
    /*
     * Compute and return signal transmission Epoch
     */
    pub(crate) fn transmission_time(&self, cfg: &Config) -> Result<Epoch, Error> {
        let (t, ts) = (self.t, self.t.time_scale);
        let seconds_ts = t.to_duration().to_seconds();
        let dt_tx = seconds_ts - self.pseudo_range().value / SPEED_OF_LIGHT;
        let mut e_tx = Epoch::from_duration(dt_tx * Unit::Second, ts);

        if cfg.modeling.sv_clock_bias {
            debug!("{:?} ({}) clock_corr: {}", t, self.sv, self.clock_corr);
            e_tx -= self.clock_corr;
        }

        if cfg.modeling.sv_total_group_delay {
            if let Some(tgd) = self.tgd {
                debug!("{:?} ({}) tgd   : {}", t, self.sv, tgd);
                e_tx -= tgd;
            }
        }

        /* run physical verification */
        let dt = (t - e_tx).to_seconds();
        assert!(dt > 0.0, "resolved t_tx is physically impossible");
        assert!(dt < 1.0, "resolved t_tx is physically impossible");
        Ok(e_tx)
    }
}
