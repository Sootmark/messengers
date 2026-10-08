//! Arbitrary bytes, and the test database damaged and cut anywhere, read
//! or are refused: never a panic.

use std::io::Read;

use proptest::prelude::*;

fn database() -> Vec<u8> {
    let mut data = Vec::new();
    common::gzip::Decoder::new(&include_bytes!("fixtures/plaso/skype_main.db.gz")[..])
        .read_to_end(&mut data)
        .unwrap();
    data
}

proptest! {
    #[test]
    fn arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = messengers::read_skype(&data, &[]);
        let _ = messengers::detect(&data);
    }

    #[test]
    fn damaged_and_cut(
        flips in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..64),
        cut in any::<usize>(),
    ) {
        let mut data = database();
        for &(at, byte) in &flips {
            let at = at % data.len();
            data[at] = byte;
        }
        data.truncate(1 + cut % data.len());
        let _ = messengers::read_skype(&data, &[]);
        let _ = messengers::detect(&data);
    }
}
