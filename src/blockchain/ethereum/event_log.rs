use web3::{
    ethabi::RawLog,
    types::{Address, U256},
};

pub trait Web3LogEvent {
    fn match_event(&self, contract: &web3::ethabi::Contract, event_name: &str) -> Option<web3::ethabi::Log>;
}

pub trait Web3ABILogEvent {
    fn get_param(&self, name: &str) -> Option<&web3::ethabi::Token>;
}
impl Web3LogEvent for web3::types::Log {
    fn match_event(&self, contract: &web3::ethabi::Contract, event_name: &str) -> Option<web3::ethabi::Log> {
        let event = contract.event(event_name).unwrap();
        let raw_log = RawLog {
            topics: self.topics.clone(),
            data: self.data.0.clone(),
        };

        if let Ok(decode_log) = event.parse_log(raw_log) {
            Some(decode_log)
        } else {
            None
        }
    }
}

impl Web3ABILogEvent for web3::ethabi::Log {
    fn get_param(&self, name: &str) -> Option<&web3::ethabi::Token> {
        if let Some(param) = self.params.iter().find(|param| param.name == name) {
            Some(&param.value)
        } else {
            None
        }
    }
}

pub trait Web3LogExt {
    fn get_address(&self, index: usize) -> Address;
    fn get_u256(&self, index: usize) -> U256;
    fn get_utf8_string(&self, index: usize) -> String;
}

impl Web3LogExt for web3::types::Log {
    fn get_address(&self, index: usize) -> Address {
        Address::from_slice(&self.data.0[index * 32 + 12..(index + 1) * 32])
    }

    fn get_u256(&self, index: usize) -> U256 {
        U256::from_big_endian(&self.data.0[index * 32..(index + 1) * 32])
    }

    fn get_utf8_string(&self, index: usize) -> String {
        String::from_utf8(self.data.0[index * 32..(index + 1) * 32].to_vec()).unwrap().trim_end_matches('\0').to_string()
    }
}
