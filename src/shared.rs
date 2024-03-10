pub static CHUNK_SIZE: usize = 16;

pub trait ModBase64 {
    type Output;
    fn fix_base64(self) -> Self::Output;
    fn fuckup_base64(self) -> Self::Output;
}

impl ModBase64 for &str {
    type Output = String;
    fn fix_base64(self) -> Self::Output {
        self.replace('~', "=").replace('!', "/").replace('-', "+")
    }

    fn fuckup_base64(self) -> Self::Output {
        self.replace('=', "~").replace('/', "!").replace('+', "-")
    }
}
