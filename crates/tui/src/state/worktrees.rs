use std::collections::HashMap;

use crate::external::{BranchPrInfo, WorktreeInfo};

pub struct WorktreesState {
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
    /// PR info indexed by branch name
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

    /// Get PR info for a branch if available
    pub fn pr_for_branch(&self, branch: &str) -> Option<&BranchPrInfo> {
        self.branch_prs.get(branch)
    }

    /// Set PR info for a branch
    pub fn set_branch_pr(&mut self, branch: String, pr_info: BranchPrInfo) {
        self.branch_prs.insert(branch, pr_info);
    }

    /// Clear PR info (e.g., when branch has no PR)
    pub fn clear_branch_pr(&mut self, branch: &str) {
        self.branch_prs.remove(branch);
    }

    pub fn set_worktrees(&mut self, worktrees: Vec<WorktreeInfo>) {
        self.worktrees = worktrees;
        self.error = None;
        // Try to keep selection on current worktree
        if let Some(idx) = self.worktrees.iter().position(|wt| wt.is_current) {
            self.selected_index = idx;
        } else if self.selected_index >= self.worktrees.len() {
            self.selected_index = self.worktrees.len().saturating_sub(1);
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
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

    pub fn current_worktree(&self) -> Option<&WorktreeInfo> {
        self.worktrees.iter().find(|wt| wt.is_current)
    }
}

impl Default for WorktreesState {
    fn default() -> Self {
        Self::new()
    }
}
