use leptos::*;
use leptos::prelude::*;
#[derive(Clone, Debug)]
pub struct TransactionData {
    pub signature: String,
    pub block_time: i64,
    pub success: bool,
    pub fee: u64,
    pub transaction_type: Option<String>,
}

#[component]
pub fn TransactionView(#[prop(into)] transaction: TransactionData) -> impl IntoView {
    // Formater la date
    let formatted_time = move || {
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(transaction.block_time, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        format!("{}", date.format("%d/%m/%Y %H:%M:%S"))
    };

    // Statut de la transaction
    let status_class = if transaction.success { "badge badge-success" } else { "badge badge-error" };
    let status_text = if transaction.success { "Succès" } else { "Échec" };

    // Formater le montant SOL
    let format_sol = |lamports: u64| -> String {
        format!("{:.9} SOL", lamports as f64 / 1_000_000_000.0)
    };

    view! {
        <div class="transaction-card card">
            <div class="transaction-header">
                <div class="transaction-signature">
                    <h3>Signature</h3>
                    <div class="signature-text">{transaction.signature.clone()}</div>
                </div>
                <div class="transaction-status">
                    <span class={status_class.to_string()}>{status_text.to_string()}</span>
                </div>
            </div>
            
            <div class="transaction-details">
                <div class="detail-item">
                    <div class="detail-label">Type:</div>
                    <div class="detail-value">{transaction.transaction_type.clone().unwrap_or_else(|| "TRANSFER".to_string())}</div>
                </div>
                
                <div class="detail-item">
                    <div class="detail-label">Date:</div>
                    <div class="detail-value">{formatted_time()}</div>
                </div>
                
                <div class="detail-item">
                    <div class="detail-label">Frais:</div>
                    <div class="detail-value">{format_sol(transaction.fee)}</div>
                </div>
            </div>
        </div>
    }
}