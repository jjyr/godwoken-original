use crate::packed;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScriptHashType {
    Data = 0,
    Type = 1,
}

impl Into<u8> for ScriptHashType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl Into<packed::Byte> for ScriptHashType {
    fn into(self) -> packed::Byte {
        (self as u8).into()
    }
}
