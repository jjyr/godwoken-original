use crate::{bytes::Bytes, cache::KVMap, packed, prelude::*, vec::Vec};

const NATIVE_TOKEN_ID: [u8; 32] = [0u8; 32];

impl Pack<packed::Byte20> for [u8; 20] {
    fn pack(&self) -> packed::Byte20 {
        packed::Byte20::from_slice(&self[..]).expect("impossible: fail to pack [u8; 20]")
    }
}

impl<'r> Unpack<[u8; 20]> for packed::Byte20Reader<'r> {
    fn unpack(&self) -> [u8; 20] {
        let ptr = self.as_slice().as_ptr() as *const [u8; 20];
        unsafe { *ptr }
    }
}
impl_conversion_for_entity_unpack!([u8; 20], Byte20);

impl Pack<packed::Byte65> for [u8; 65] {
    fn pack(&self) -> packed::Byte65 {
        packed::Byte65::from_slice(&self[..]).expect("impossible: fail to pack [u8; 65]")
    }
}

impl<'r> Unpack<[u8; 65]> for packed::Byte65Reader<'r> {
    fn unpack(&self) -> [u8; 65] {
        let ptr = self.as_slice().as_ptr() as *const [u8; 65];
        unsafe { *ptr }
    }
}
impl_conversion_for_entity_unpack!([u8; 65], Byte65);

impl Pack<packed::KeyValueMap> for KVMap {
    fn pack(&self) -> packed::KeyValueMap {
        let mut builder = packed::KeyValueMapBuilder::default();
        for (k, v) in self.iter() {
            let kv_pair = packed::KeyValueBuilder::default()
                .key(k.pack())
                .value(v.pack())
                .build();
            builder = builder.push(kv_pair);
        }
        builder.build()
    }
}

impl<'r> Unpack<KVMap> for packed::KeyValueMapReader<'r> {
    fn unpack(&self) -> KVMap {
        let mut kv_map = KVMap::default();
        for i in 0..self.item_count() {
            let kv_pair = self.get(i).unwrap();
            let key: [u8; 32] = kv_pair.key().unpack();
            let value: u64 = kv_pair.value().unpack();
            kv_map.insert(key, value);
        }
        kv_map
    }
}

impl Pack<packed::SMTBranch> for ([u8; 32], u8) {
    fn pack(&self) -> packed::SMTBranch {
        let node: packed::Byte32 = self.0.pack();
        let builder = packed::SMTBranchBuilder::default();
        builder.node(node).height(self.1.into()).build()
    }
}

impl<'r> Unpack<([u8; 32], u8)> for packed::SMTBranchReader<'r> {
    fn unpack(&self) -> ([u8; 32], u8) {
        let node: [u8; 32] = self.node().unpack();
        let height: u8 = self.height().into();
        (node, height)
    }
}

impl_conversion_for_entity_unpack!(([u8; 32], u8), SMTBranch);
impl_conversion_for_vector!(([u8; 32], u8), SMTBranchVec, SMTBranchVecReader);

impl Pack<packed::TreePath> for Vec<u8> {
    fn pack(&self) -> packed::TreePath {
        let len = self.len();
        let mut vec: Vec<u8> = Vec::with_capacity(4 + len);
        vec.extend_from_slice(&(len as u32).to_le_bytes()[..]);
        vec.extend_from_slice(self);
        packed::TreePath::new_unchecked(Bytes::from(vec))
    }
}

impl<'r> Unpack<Vec<u8>> for packed::TreePathReader<'r> {
    fn unpack(&self) -> Vec<u8> {
        self.raw_data().to_vec()
    }
}

impl_conversion_for_entity_unpack!(Vec<u8>, TreePath);
impl_conversion_for_vector!(Vec<u8>, TreePathVec, TreePathVecReader);

impl Pack<packed::Payment> for ([u8; 32], u64) {
    fn pack(&self) -> packed::Payment {
        let inner = if self.0 == NATIVE_TOKEN_ID {
            packed::PaymentUnion::Uint32((self.1 as u32).pack())
        } else {
            packed::PaymentUnion::UDTPayment(
                packed::UDTPayment::new_builder()
                    .type_hash(self.0.pack())
                    .amount((self.1 as u32).pack())
                    .build(),
            )
        };
        packed::Payment::new_builder().set(inner).build()
    }
}

impl<'r> Unpack<([u8; 32], u64)> for packed::PaymentReader<'r> {
    fn unpack(&self) -> ([u8; 32], u64) {
        match self.to_enum() {
            packed::PaymentUnionReader::Uint32(amount) => {
                let amount: u32 = amount.unpack();
                (NATIVE_TOKEN_ID, amount.into())
            }
            packed::PaymentUnionReader::UDTPayment(udt_payment) => {
                let udt_type: [u8; 32] = udt_payment.type_hash().unpack();
                let amount: u32 = udt_payment.amount().unpack();
                (udt_type, amount.into())
            }
        }
    }
}

impl_conversion_for_entity_unpack!(([u8; 32], u64), Payment);
