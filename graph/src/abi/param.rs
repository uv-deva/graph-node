use alloy::dyn_abi::DynSolValue;

#[derive(Clone, Debug)]
pub struct DynSolParam {
    pub name: String,
    pub value: DynSolValue,
}
