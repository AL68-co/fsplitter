pub fn add_one(inp: u32) -> u32 {
    inp + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(add_one(0), 1);
    }
}
