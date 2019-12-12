// pack data to ckb_types
macro_rules! to {
    ($entity:expr) => {
        ckb_types::prelude::Pack::pack(&godwoken_types::prelude::Unpack::unpack($entity))
    };
    ($entity:expr, $t:ty) => {
        ckb_types::packed::$t::new_unchecked($entity.as_slice().to_vec().into())
    };
}

pub fn merkle_root(leaves: &[godwoken_types::packed::Byte32]) -> godwoken_types::packed::Byte32 {
    let leaves: Vec<ckb_types::packed::Byte32> = leaves.iter().map(|leaf| to!(leaf)).collect();
    let root: ckb_types::H256 =
        ckb_types::prelude::Unpack::unpack(&ckb_types::utilities::merkle_root(leaves.as_slice()));
    godwoken_types::prelude::Pack::pack(&root.0)
}
