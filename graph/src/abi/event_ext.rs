use alloy::json_abi::Event;
use alloy::primitives::LogData;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use web3::types::Log;

use crate::abi::DynSolParam;

pub trait EventExt {
    fn decode_log(&self, log: &Log) -> Result<Vec<DynSolParam>>;
}

impl EventExt for Event {
    fn decode_log(&self, log: &Log) -> Result<Vec<DynSolParam>> {
        let log_data = log_to_log_data(log)?;
        let decoded_event = alloy::dyn_abi::EventExt::decode_log(self, &log_data, true)?;

        if self.inputs.len() != decoded_event.indexed.len() + decoded_event.body.len() {
            return Err(anyhow!(
                "unexpected number of decoded event inputs; expected {}, got {}",
                self.inputs.len(),
                decoded_event.indexed.len() + decoded_event.body.len(),
            ));
        }

        let decoded_params = decoded_event
            .indexed
            .into_iter()
            .chain(decoded_event.body.into_iter())
            .enumerate()
            .map(|(i, value)| DynSolParam {
                name: self.inputs[i].name.clone(),
                value,
            })
            .collect();

        Ok(decoded_params)
    }
}

fn log_to_log_data(log: &Log) -> Result<LogData> {
    let topics = log
        .topics
        .iter()
        .map(|x| x.to_fixed_bytes().into())
        .collect_vec();

    let data = log.data.0.clone().into();

    LogData::new(topics, data).context("log has an invalid number of topics")
}
