use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ScoreAccount {
    pub owner: Pubkey,
    pub score: u16,
    pub on_time_payments: u32,
    pub late_payments: u32,
    pub defaults: u16,
    pub total_loans: u32,
    pub last_updated: i64,
    pub bump: u8,
}

impl ScoreAccount {
    pub const SIZE: usize = 32 + 2 + 4 + 4 + 2 + 4 + 8 + 1; // 57 bytes
    
    pub fn new(
        owner: Pubkey,
        initial_score: u16,
        created_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            owner,
            score: initial_score,
            on_time_payments: 0,
            late_payments: 0,
            defaults: 0,
            total_loans: 0,
            last_updated: created_at,
            bump,
        }
    }
    
    pub fn update_score(&mut self, change: i16, current_time: i64) {
        if change > 0 {
            // Augmenter le score, maximum 1000
            let new_score = self.score.saturating_add(change as u16);
            self.score = std::cmp::min(new_score, 1000);
            
            // Mettre à jour les statistiques de paiement
            self.on_time_payments = self.on_time_payments.saturating_add(1);
        } else if change < -30 {
            // Défaut de paiement (pénalité forte)
            self.score = self.score.saturating_sub(change.abs() as u16);
            self.defaults = self.defaults.saturating_add(1);
        } else if change < 0 {
            // Paiement en retard (pénalité moyenne)
            self.score = self.score.saturating_sub(change.abs() as u16);
            self.late_payments = self.late_payments.saturating_add(1);
        }
        
        // Mettre à jour la date de dernière mise à jour
        self.last_updated = current_time;
    }
    
    pub fn record_new_loan(&mut self, current_time: i64) {
        self.total_loans = self.total_loans.saturating_add(1);
        self.last_updated = current_time;
    }
}