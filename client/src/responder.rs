use async_std::{fs, io::ReadExt};
use proto::prelude::ID;


struct Responder;

const PATH:&'static str="host_save";
// const PATH: &'static Path = Path::new("./");

async fn get_uid()->ID{
    todo!();
    // if fs::File:;exs
    let mut file=fs::File::open(PATH).await.unwrap();

    let mut buf=Vec::new();
    file.read_to_end(&mut buf).await.unwrap();

    let id:ID=bincode::deserialize(&buf).unwrap();

    id
}


// no buffer, interact directly with underlying storage
// However, grub-query(disk scan should only 'take once')
impl Responder {
    async fn respond_handshake(){

    }
}

