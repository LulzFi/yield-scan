use web3::types::{Address, H256, U256};

pub trait HexParseTrait {
    fn to_hex_string(&self) -> String;
    fn to_0x_string(&self) -> String;
    fn from_0x_string(s: &str) -> Self;
}

impl HexParseTrait for H256 {
    fn to_hex_string(&self) -> String {
        format!("{:#x}", self)
    }
    fn to_0x_string(&self) -> String {
        format!("0x{:#x}", self)
    }
    
    fn from_0x_string(s: &str) -> Self {
        H256::from_slice(&hex::decode(&s[2..]).unwrap())
    }
    
}

impl HexParseTrait for Address {
    fn to_hex_string(&self) -> String {
        format!("{:#x}", self)
    }
    fn to_0x_string(&self) -> String {
        format!("0x{:#x}", self)
    }
    fn from_0x_string(s: &str) -> Self {
        Address::from_slice(&hex::decode(&s[2..]).unwrap())
    }
}

pub trait FromWeiTrait {
    fn from_wei(&self, decimals: u64) -> f64;
}

impl FromWeiTrait for U256 {
    fn from_wei(&self, decimals: u64) -> f64 {
        let divisor = U256::from(10).pow(U256::from(decimals));
        let numerator = self.as_u128() as f64;
        let denominator = divisor.as_u128() as f64;
        numerator / denominator
    }
}
