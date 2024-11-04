use std::borrow::Cow;

use alloy::dyn_abi::DynSolType;
use alloy::dyn_abi::DynSolValue;
use alloy::dyn_abi::Specifier;
use alloy::json_abi::Function;
use alloy::json_abi::Param;
use anyhow::anyhow;
use anyhow::Result;
use itertools::Itertools;

use crate::abi::DynSolValueExt;

pub trait FunctionExt {
    /// Returns the signature of this function in the following formats:
    /// - if the function has no outputs: `$name($($inputs),*)`
    /// - if the function has outputs: `$name($($inputs),*):($(outputs),*)`
    ///
    /// Examples:
    /// - `functionName()`
    /// - `functionName():(uint256)`
    /// - `functionName(bool):(uint256,string)`
    /// - `functionName(uint256,bytes32):(string,uint256)`
    fn signature_compat(&self) -> String;

    /// ABI-decodes the given data according to the function's input types.
    fn abi_decode_input(&self, data: &[u8]) -> Result<Vec<DynSolValue>>;

    /// ABI-decodes the given data according to the function's output types.
    fn abi_decode_output(&self, data: &[u8]) -> Result<Vec<DynSolValue>>;

    /// ABI-encodes the given values, prefixed by the function's selector, if any.
    fn abi_encode_input(&self, values: &[DynSolValue]) -> Result<Vec<u8>>;
}

impl FunctionExt for Function {
    fn signature_compat(&self) -> String {
        let name = &self.name;
        let inputs = &self.inputs;
        let outputs = &self.outputs;

        // This is what `alloy` uses internally when creating signatures.
        const MAX_SOL_TYPE_LEN: usize = 32;

        let mut sig_cap = name.len() + 1 + inputs.len() * MAX_SOL_TYPE_LEN + 1;

        if !outputs.is_empty() {
            sig_cap = sig_cap + 2 + outputs.len() * MAX_SOL_TYPE_LEN + 1;
        }

        let mut sig = String::with_capacity(sig_cap);

        sig.push_str(&name);
        signature_part(&inputs, &mut sig);

        if !outputs.is_empty() {
            sig.push(':');
            signature_part(&outputs, &mut sig);
        }

        sig
    }

    fn abi_decode_input(&self, data: &[u8]) -> Result<Vec<DynSolValue>> {
        (self as &dyn alloy::dyn_abi::FunctionExt)
            .abi_decode_input(data, true)
            .map_err(Into::into)
    }

    fn abi_decode_output(&self, data: &[u8]) -> Result<Vec<DynSolValue>> {
        (self as &dyn alloy::dyn_abi::FunctionExt)
            .abi_decode_output(data, true)
            .map_err(Into::into)
    }

    fn abi_encode_input(&self, values: &[DynSolValue]) -> Result<Vec<u8>> {
        let inputs = &self.inputs;

        if inputs.len() != values.len() {
            return Err(anyhow!(
                "unexpected number of values; expected {}, got {}",
                inputs.len(),
                values.len(),
            ));
        }

        let mut fixed_values = Vec::with_capacity(values.len());

        for (i, input) in inputs.iter().enumerate() {
            let ty = input.resolve()?;
            let val = &values[i];

            fixed_values.push(fix_type_size(&ty, val)?);
        }

        if fixed_values.iter().all(|x| matches!(x, Cow::Borrowed(_))) {
            return (self as &dyn alloy::dyn_abi::JsonAbiExt)
                .abi_encode_input(values)
                .map_err(Into::into);
        }

        // Required because of `alloy::dyn_abi::JsonAbiExt::abi_encode_input` API;
        let owned_fixed_values = fixed_values
            .into_iter()
            .map(|x| x.into_owned())
            .collect_vec();

        (self as &dyn alloy::dyn_abi::JsonAbiExt)
            .abi_encode_input(&owned_fixed_values)
            .map_err(Into::into)
    }
}

// An efficient way to compute a part of the signature without new allocations.
fn signature_part(params: &[Param], out: &mut String) {
    out.push('(');

    match params.len() {
        0 => {}
        1 => {
            params[0].selector_type_raw(out);
        }
        n => {
            params[0].selector_type_raw(out);

            for i in 1..n {
                out.push(',');
                params[i].selector_type_raw(out);
            }
        }
    }

    out.push(')');
}

// Alloy is stricter in type checking than `ehtabi` and requires that the decoded values have
// exactly the same number of bits / bytes as the type used for checking.
//
// This is a problem because in some ASC conversions we lose the original number of bits / bytes
// if the actual data takes less memory.
//
// This method fixes that in a simple but not very cheap way, by encoding the value and trying
// to decode it again using the given type. The result fixes the number of bits / bytes in the
// decoded values, so we can use `alloy` methods that have strict type checking internally.
fn fix_type_size<'a>(ty: &DynSolType, val: &'a DynSolValue) -> Result<Cow<'a, DynSolValue>> {
    if val.matches(ty) {
        return Ok(Cow::Borrowed(val));
    }

    if !val.type_check(ty) {
        return Err(anyhow!(
            "invalid value type; expected '{}', got '{:?}'",
            ty.sol_type_name(),
            val.sol_type_name(),
        ));
    }

    let bytes = val.abi_encode();
    let new_val = ty.abi_decode(&bytes)?;

    Ok(Cow::Owned(new_val))
}
