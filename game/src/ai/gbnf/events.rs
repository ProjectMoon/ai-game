// use crate::models::commands::ChangeScene;

// use super::Gbnf;

// // TODO put all events in one place and change root based on event
const CHANGE_SCENE_BNF: &'static str = r#"
root ::= ChangeScene
ChangeScene ::= "{"   ws   "\"scenekey\":"   ws   string   "}"
ChangeScenelist ::= "[]" | "["   ws   ChangeScene   (","   ws   ChangeScene)*   "]"
string ::= "\""   ([^"]*)   "\""
boolean ::= "true" | "false"
ws ::= [ \t\n]*
number ::= [0-9]+   "."?   [0-9]*
stringlist ::= "["   ws   "]" | "["   ws   string   (","   ws   string)*   ws   "]"
numberlist ::= "["   ws   "]" | "["   ws   string   (","   ws   number)*   ws   "]"
"#;

// impl Gbnf for ChangeScene {
//     fn to_gbnf() -> String {
//         CHANGE_SCENE_BNF.to_string()
//     }
// }
