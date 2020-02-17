use serde::{Serialize, Deserialize};
use crate::*;
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
pub enum ClientCommand {
    Join,
    GetAll,
    Op(DocumentOp)
}
impl ClientCommand {
    pub fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
    pub fn decode(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

#[derive(Serialize, Deserialize)]
pub enum ServerCommand<'a> {
    Welcome(SiteId),
    Document(Cow<'a, GlobalDocument>),
    Op(Cow<'a, DocumentOp>)
}
impl<'a> ServerCommand<'a> {
    pub fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
impl ServerCommand<'static> {
    pub fn decode(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}
/* client -[Join]-> server
         <-[Welcome]-
         -[GetAll]->
        <-[Document]-

        <-[Op]->
*/