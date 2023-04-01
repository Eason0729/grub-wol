use std::path::Path;

use async_std::{fs::File, io::{WriteExt, ReadExt}};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait AsyncState<O>
where
    Self: for<'a> Deserialize<'a> + Serialize + Default,
    O: Sync,
{
    async fn serde(machine: &O) -> Self;
    fn deserde(self) -> O;
    async fn load(path: &Path) -> O {
        let save = if path.exists() && path.is_file() {
            let mut file = File::open(path).await.unwrap();

            let buf = &mut Vec::new();
            file.read_to_end(buf).await.unwrap();

            bincode::deserialize::<Self>(buf).unwrap()
        } else {
            Default::default()
        };
        save.deserde()
    }
    async fn save(src: &O, path: &Path) {
        let buf = bincode::serialize(&Self::serde(src).await).unwrap();

        log::trace!("Serialized save file");
        let mut file = File::open(path).await.unwrap();
        file.write_all(&buf).await.unwrap();
        log::info!("Saving Done");
    }
}

struct State{

}