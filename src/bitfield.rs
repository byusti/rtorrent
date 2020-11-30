pub type Bitfield = Vec<u8>;

pub(crate) fn has_piece(bf: &mut Bitfield, index: usize) -> bool {
    let byte_index = index / 8;
    let offset = index % 8;
    let bflength = bf.len() as usize;
    if byte_index >= bflength {
        false
    } else {
        bf[byte_index] >> (7 - offset) as u8 & 1 != 0
    }
}

pub fn set_piece(bf: &Bitfield, index: usize) -> Bitfield {
    let byte_index = index / 8;
    let offset = index % 8;
    let bflength = bf.len() as usize;
    let mut newbf = bf.to_vec();
    if byte_index >= bflength {
        newbf
    } else {
        newbf[byte_index] |= (1 << (7 - offset)) as u8;
        newbf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_piece() {
        let mut bf: Bitfield = vec![0b01010100, 0b01010100];
        let outputs: [bool; 20] = [
            false, true, false, true, false, true, false, false, false, true, false, true, false,
            true, false, false, false, false, false, false,
        ];
        for i in 0..outputs.len() {
            assert_eq!(outputs[i], has_piece(&mut bf, i))
        }
    }

    #[test]
    fn test_set_piece() {
        #[derive(Clone)]
        struct Test {
            input: Bitfield,
            index: usize,
            output: Bitfield,
        }
        let tests: [Test; 4] = [
            Test {
                input: vec![0b01010100, 0b01010100],
                index: 4,
                output: vec![0b01011100, 0b01010100],
            },
            Test {
                input: vec![0b01010100, 0b01010100],
                index: 9,
                output: vec![0b01010100, 0b01010100],
            },
            Test {
                input: vec![0b01010100, 0b01010100],
                index: 15,
                output: vec![0b01010100, 0b01010101],
            },
            Test {
                input: vec![0b01010100, 0b01010100],
                index: 19,
                output: vec![0b01010100, 0b01010100],
            },
        ];
        assert_eq!(tests[0].output, set_piece(&tests[0].input, tests[0].index));
        assert_eq!(tests[1].output, set_piece(&tests[1].input, tests[1].index));
        assert_eq!(tests[2].output, set_piece(&tests[2].input, tests[2].index));
        assert_eq!(tests[3].output, set_piece(&tests[3].input, tests[3].index));
    }
}
