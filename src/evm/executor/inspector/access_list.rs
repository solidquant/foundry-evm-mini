use ethers::{
    abi::{ethereum_types::BigEndianHash, Address},
    types::{
        transaction::eip2930::{AccessList, AccessListItem},
        H160, H256,
    },
};
use hashbrown::{HashMap, HashSet};
use revm::{
    interpreter::{opcode, InstructionResult, Interpreter},
    precompile::Precompiles,
    primitives::SpecId,
    Database, EVMData, Inspector,
};
use std::fmt;

use crate::evm::utils::{b160_to_h160, ru256_to_u256};

pub fn get_precompiles_for(spec_id: SpecId) -> Vec<Address> {
    Precompiles::new(to_precompile_id(spec_id))
        .addresses()
        .into_iter()
        .map(|item| H160::from_slice(item))
        .collect()
}

/// wrapper type that displays byte as hex
pub struct HexDisplay<'a>(&'a [u8]);

pub fn hex_fmt_many<I, T>(i: I) -> String
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    i.into_iter()
        .map(|item| HexDisplay::from(item.as_ref()).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl<'a> HexDisplay<'a> {
    pub fn from(b: &'a [u8]) -> Self {
        HexDisplay(b)
    }
}

impl<'a> fmt::Display for HexDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.len() < 1027 {
            for byte in self.0 {
                f.write_fmt(format_args!("{byte:02x}"))?;
            }
        } else {
            for byte in &self.0[0..512] {
                f.write_fmt(format_args!("{byte:02x}"))?;
            }
            f.write_str("...")?;
            for byte in &self.0[self.0.len() - 512..] {
                f.write_fmt(format_args!("{byte:02x}"))?;
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for HexDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

pub fn to_precompile_id(spec_id: SpecId) -> revm::precompile::SpecId {
    match spec_id {
        SpecId::FRONTIER
        | SpecId::FRONTIER_THAWING
        | SpecId::HOMESTEAD
        | SpecId::DAO_FORK
        | SpecId::TANGERINE
        | SpecId::SPURIOUS_DRAGON => revm::precompile::SpecId::HOMESTEAD,
        SpecId::BYZANTIUM | SpecId::CONSTANTINOPLE | SpecId::PETERSBURG => {
            revm::precompile::SpecId::BYZANTIUM
        }
        SpecId::ISTANBUL | SpecId::MUIR_GLACIER => revm::precompile::SpecId::ISTANBUL,
        SpecId::BERLIN
        | SpecId::LONDON
        | SpecId::ARROW_GLACIER
        | SpecId::GRAY_GLACIER
        | SpecId::MERGE
        | SpecId::SHANGHAI
        | SpecId::CANCUN
        | SpecId::LATEST => revm::precompile::SpecId::BERLIN,
    }
}

/// An inspector that collects touched accounts and storage slots.
#[derive(Default, Debug)]
pub struct AccessListTracer {
    excluded: HashSet<Address>,
    access_list: HashMap<Address, HashSet<H256>>,
}

impl AccessListTracer {
    pub fn new(
        access_list: AccessList,
        from: Address,
        to: Address,
        precompiles: Vec<Address>,
    ) -> Self {
        AccessListTracer {
            excluded: [from, to]
                .iter()
                .chain(precompiles.iter())
                .copied()
                .collect(),
            access_list: access_list
                .0
                .iter()
                .map(|v| (v.address, v.storage_keys.iter().copied().collect()))
                .collect(),
        }
    }

    pub fn access_list(&self) -> AccessList {
        AccessList::from(
            self.access_list
                .iter()
                .map(|(address, slots)| AccessListItem {
                    address: *address,
                    storage_keys: slots.iter().copied().collect(),
                })
                .collect::<Vec<AccessListItem>>(),
        )
    }
}

impl<DB: Database> Inspector<DB> for AccessListTracer {
    #[inline]
    fn step(
        &mut self,
        interpreter: &mut Interpreter,
        _data: &mut EVMData<'_, DB>,
    ) -> InstructionResult {
        let pc = interpreter.program_counter();
        let op = interpreter.contract.bytecode.bytecode()[pc];

        match op {
            opcode::SLOAD | opcode::SSTORE => {
                if let Ok(slot) = interpreter.stack().peek(0) {
                    let cur_contract = interpreter.contract.address;
                    self.access_list
                        .entry(b160_to_h160(cur_contract))
                        .or_default()
                        .insert(H256::from_uint(&ru256_to_u256(slot)));
                }
            }
            opcode::EXTCODECOPY
            | opcode::EXTCODEHASH
            | opcode::EXTCODESIZE
            | opcode::BALANCE
            | opcode::SELFDESTRUCT => {
                if let Ok(slot) = interpreter.stack().peek(0) {
                    let addr: Address = H256::from_uint(&ru256_to_u256(slot)).into();
                    if !self.excluded.contains(&addr) {
                        self.access_list.entry(addr).or_default();
                    }
                }
            }
            opcode::DELEGATECALL | opcode::CALL | opcode::STATICCALL | opcode::CALLCODE => {
                if let Ok(slot) = interpreter.stack().peek(1) {
                    let addr: Address = H256::from_uint(&ru256_to_u256(slot)).into();
                    if !self.excluded.contains(&addr) {
                        self.access_list.entry(addr).or_default();
                    }
                }
            }
            _ => (),
        }

        InstructionResult::Continue
    }
}
