// 自动生成；请修改 packages/contracts/src/schema.ts 后运行 contracts:v2:generate。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommandKind {
    #[serde(rename = "photo.set_selection")]
    PhotoSetSelection,
    #[serde(rename = "sequence.move_item")]
    SequenceMoveItem,
    #[serde(rename = "sequence.add_item")]
    SequenceAddItem,
    #[serde(rename = "sequence.remove_item")]
    SequenceRemoveItem,
    #[serde(rename = "proposal.create")]
    ProposalCreate,
    #[serde(rename = "proposal.submit")]
    ProposalSubmit,
    #[serde(rename = "proposal.apply")]
    ProposalApply,
    #[serde(rename = "comment.add")]
    CommentAdd,
    #[serde(rename = "comment.resolve")]
    CommentResolve,
}
