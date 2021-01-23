// #[derive(Debug, Clone)]
// pub enum Target {
//     Proof(MarkedStatement),
//     Calculate(Statement),
//     Transform,
// }
//
// pub struct TargetBuilder<'a> {
//     node:   &'a ParserNode,
//     params: Option<&'a mut ParamsMap>,
// }
//
// impl Target {
//     pub fn try_from(node: &ParserNode, params: &mut ParamsMap) ->
// Result<Self, String> {         if node.degree() != 1 {
//             return Err("Wrong target tree".into());
//         }
//         let label = node.first().unwrap();
//
//         match label.data.as_ref() {
//             "proof" => {
//                 if label.degree() != 1 {
//                     return Err(format!("Wrong target tree {:?}", label));
//                 }
//                 let target = Statement::new(label.first().unwrap(), params)?;
//
//                 
// Ok(ProblemType::Proof(MarkedStatement::from(Arc::new(target))))             }
//             "find" => {
//                 if label.degree() != 1 {
//                     return Err(format!("Wrong target tree {:?}", label));
//                 }
//                 let target = Statement::new(label.first().unwrap(), params)?;
//
//                 Ok(ProblemType::Calculate(target.root))
//             }
//             "transform" => Ok(ProblemType::Transform),
//             _ => Err(format!("Incorrect problem type: {}", label.data)),
//         }
//     }
//
//     pub fn map(&self, other: &Self) -> Result<Vec<TreeParamsMap>, String> {
//         match (self, other) {
//             (ProblemType::Calculate(x), ProblemType::Calculate(y)) =>
// params_map(&x, &y),             (ProblemType::Proof(x),
// ProblemType::Proof(y)) => {                 params_map(&x.statement.root,
// &y.statement.root)             }
//             (ProblemType::Transform, ProblemType::Transform) => Ok(vec![]),
//             _ => Err("Targets is differ".into()),
//         }
//     }
// }
//
// impl<'a> TargetBuilder<'a> {
//     pub fn new(node: &'a ParserNode) -> Self {
//         Self {
//             node:   node,
//             params: None,
//         }
//     }
//
//     pub fn with_params(mut self, params: &'a mut ParamsMap) -> Self {
//         self.params = Some(params);
//         self
//     }
//
//     pub fn statement(mut self) -> Result<ProblemType, String> {
//         self.parse_problem_type(NodeType::Statement)
//     }
//
//     pub fn rule(mut self) -> Result<ProblemType, String> {
//         self.parse_problem_type(NodeType::Rule)
//     }
//
//     fn parse_problem_type(&mut self, node_type: NodeType) ->
// Result<ProblemType, String> {         let mut empty_map = ParamsMap::new();
//         let params = self.params.take().unwrap_or(&mut empty_map);
//         if self.node.degree() != 1 {
//             return Err("Wrong target tree".into());
//         }
//         let label = self.node.first().unwrap();
//
//         if label.data == "transform" {
//             return Ok(ProblemType::Transform);
//         }
//
//         if label.degree() != 1 {
//             return Err(format!("Wrong target tree {:?}", label));
//         }
//         let target = match node_type {
//             NodeType::Statement => Statement::new(label.first().unwrap(),
// params)?,             NodeType::Rule =>
// Statement::new_with_params(label.first().unwrap(), params)?,         };
//
//         match label.data.as_ref() {
//             "proof" =>
// Ok(ProblemType::Proof(MarkedStatement::from(Arc::new(target)))),             
// "find" => Ok(ProblemType::Calculate(target.root)),             "transform" =>
// Ok(ProblemType::Transform),             _ => Err(format!("Incorrect problem
// type: {}", label.data)),         }
//     }
// }
//
// impl fmt::Display for Target {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match self {
//             ProblemType::Proof(target) => write!(f, "proof {}",
// target.statement),             ProblemType::Calculate(target) => write!(f,
// "find {}", target),             ProblemType::Transform => write!(f,
// "transform"),         }
//     }
// }
//
