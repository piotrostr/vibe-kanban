use std::collections::HashMap;

use crate::external::{BranchPrInfo, WorktreeInfo};

pub struct WorktreesState {
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub branch_prs: HashMap<String, BranchPrInfo>,
}

impl WorktreesState {
    pub fn new() -> Self {
        Self {
            worktrees: Vec::new(),
            selected_index: 0,
            loading: false,
            error: None,
            branch_prs: HashMap::new(),
        }
    }

    pub fn pr_for_branch(&self, branch: &str) -> Option<&BranchPrInfo> {
        self.branch_prs.get(branch)
    }

    pub fn set_branch_pr(&mut self, branch: String, pr_info: BranchPrInfo) {
        self.branch_prs.insert(branch, pr_info);
    }

    pub fn clear_branch_pr(&mut self, branch: &str) {
        self.branch_prs.remove(branch);
    }

    pub fn set_worktrees(&mut self, worktrees: Vec<WorktreeInfo>) {
        self.worktrees = worktrees;
        self.error = None;
        if let Some(idx) = self.worktrees.iter().position(|wt| wt.is_current) {
            self.selected_index = idx;
        } else if self.selected_index >= self.worktrees.len() {
            self.selected_index = self.worktrees.len().saturating_sub(1);
        }
    }

    pub fn selected(&self) -> Option<&WorktreeInfo> {
        self.worktrees.get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        if !self.worktrees.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.worktrees.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.worktrees.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.worktrees.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
}

impl Default for WorktreesState {
    fn default() -> Self {
        Self::new()
    }
}
