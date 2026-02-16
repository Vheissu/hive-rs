pub trait HiveSerialize {
    fn hive_serialize(&self, _buf: &mut Vec<u8>);
}
