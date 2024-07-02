use serde_json::Value;
use eframe::egui;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Default)]
pub struct App {
    helius_api_key: String,
    transactions: Arc<Mutex<Vec<Value>>>,
    transactions_len: usize,
    getting_txns: Arc<Mutex<bool>>,
    connections: Vec<Value>,
    active_txn: Option<Value>,
    search_sig: String,
    tab: Tab,
    address: String,
    second_address: String
}

#[derive(PartialEq)]
enum Tab {
    GetTransactions,
    Search,
    ActiveTransaction,
    FindConnections
}

impl Default for Tab {
    fn default() -> Self {
        Tab::GetTransactions
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::GetTransactions, "Get Transactions");
                ui.selectable_value(&mut self.tab, Tab::Search, "Search");
                ui.selectable_value(&mut self.tab, Tab::ActiveTransaction, "Transaction");
                ui.selectable_value(&mut self.tab, Tab::FindConnections, "Find Connections");
            });
            ui.separator();

            match self.tab {
                Tab::GetTransactions => get_transactions_ui(self, ui),
                Tab::Search => search_ui(self, ui),
                Tab::ActiveTransaction => active_transaction_ui(self, ui),
                Tab::FindConnections => find_connections_ui(self, ui)
            }
        });
    }
}

fn get_transactions_ui(app: &mut App, ui: &mut egui::Ui) {
    egui::Grid::new("get-transactions-grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Helius API Key");
            ui.text_edit_singleline(&mut app.helius_api_key);
            ui.end_row();

            ui.label("Solana Address");
            ui.text_edit_singleline(&mut app.address);
            ui.end_row();
        });

    ui.horizontal(|ui| {
        if ui.button("Get Transactions").clicked() {
            let address = app.address.clone();
            let api_key = app.helius_api_key.clone();
            let txns_clone = app.transactions.clone();
            let getting_txns = app.getting_txns.clone();

            *getting_txns.lock().unwrap() = true;

            let _ = thread::spawn(move || {
                if let Ok(txns) = get_transactions(&address, &api_key, None) {
                    let mut last_txn = String::from(
                        txns[txns.len() - 1].get("signature")
                            .unwrap()
                            .as_str()
                            .unwrap()
                    );

                    *txns_clone.lock().unwrap() = txns;

                    while let Ok(mut txns) = get_transactions(&address, &api_key, Some(last_txn.clone())) {
                        if txns.len() < 1 {
                            break;
                        }

                        last_txn = String::from(
                            txns[txns.len() - 1].get("signature")
                                .unwrap()
                                .as_str()
                                .unwrap()
                        );

                        txns_clone.lock()
                            .unwrap()
                            .append(&mut txns);
                    }
                }

                *getting_txns.lock().unwrap() = false;
            });
        }

        if *app.getting_txns.lock().unwrap() {
            ui.spinner();
        }
    });

    ui.separator();

    if let Ok(txns_lock) = app.transactions.try_lock() {
        app.transactions_len = txns_lock.len();
    } 

    ui.label(format!("Retrieved {} Transactions", app.transactions_len));

    ui.separator();
    egui::CollapsingHeader::new("Transactions").show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.separator();

            if let Ok(txns_lock) = app.transactions.try_lock() {
                for txn in txns_lock.iter() {
                    let sig = txn.get("signature")
                        .unwrap()
                        .as_str()
                        .unwrap();

                    if ui.label(sig).clicked() {
                        app.active_txn = Some(txn.clone());
                        app.tab = Tab::ActiveTransaction;
                    }

                    ui.separator();
                }
            }
        });
    });
}

fn search_ui(app: &mut App, ui: &mut egui::Ui) {
    egui::Grid::new("search-grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Signature");
            ui.text_edit_singleline(&mut app.search_sig);
            ui.end_row();
        });

    if ui.button("Get Transaction").clicked() {
        if let Ok(txn) = get_transaction(app) {
            app.active_txn = Some(txn[0].clone());
            app.tab = Tab::ActiveTransaction;
        }
    }
}

fn active_transaction_ui(app: &mut App, ui: &mut egui::Ui) {
    if let None = app.active_txn {
        return;
    } 

    let txn = app.active_txn.as_ref().unwrap();
    let account_data = txn.get("accountData").unwrap();
    let sig = txn.get("signature")
        .unwrap()
        .as_str()
        .unwrap();
    let sol_bal_change = account_data[0].get("nativeBalanceChange")
        .unwrap()
        .as_i64()
        .unwrap();
    let accounts: Vec<&str> = account_data.as_array()
        .unwrap()
        .iter()
        .map(|account| {
            account.get("account")
                .unwrap()
                .as_str()
                .unwrap()
        }).collect();

    ui.heading("Transaction Info");
    ui.separator();

    ui.label(format!("Signature: {sig}"));
    ui.label(format!("Signer SOL Balance Change: {}", lamports_to_sol(sol_bal_change)));

    ui.separator();
    ui.heading("Accounts");
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for account in accounts {
            ui.label(account);
        }
    });
}

fn find_connections_ui(app: &mut App, ui: &mut egui::Ui) {
    egui::Grid::new("find-connections-grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Second Address");
            ui.text_edit_singleline(&mut app.second_address);
            ui.end_row();
        });

    if ui.button("Find Connections").clicked() {
        find_connections(app);
    }

    if app.connections.len() < 1 {
        return;
    }

    ui.separator();
    ui.heading("Connections");
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for conn in &app.connections {
            let sig = conn.get("signature")
                .unwrap()
                .as_str()
                .unwrap();

            if ui.label(sig).clicked() {
                app.active_txn = Some(conn.clone());
                app.tab = Tab::ActiveTransaction;
            }

            ui.separator();
        }
    });
}

fn find_connections(app: &mut App) {
    app.connections = Vec::new();

    for txn in app.transactions.lock().unwrap().iter() {
        let account_data = txn.get("accountData").unwrap();
        let accounts: Vec<&str> = account_data.as_array()
            .unwrap()
            .iter()
            .map(|account| {
                account.get("account")
                    .unwrap()
                    .as_str()
                    .unwrap()
            }).collect();

        for account in accounts {
            if account == app.second_address {
                app.connections.push(txn.clone());
            }
        }
    }
}

fn get_transactions(address: &str, api_key: &str, before: Option<String>) -> Result<Vec<Value>, serde_json::Error> {
    let (tx, mut rx) = mpsc::channel(1);
    let mut url = format!("https://api.helius.xyz/v0/addresses/{}/transactions?api-key={}", address, api_key);
    
    if let Some(before) = before {
        url += format!("&before={before}").as_str();
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let response = reqwest::get(url).await.unwrap();
            let data = response.text().await.unwrap();
            let json = serde_json::from_str(data.as_str());

            tx.send(json).await.unwrap();
        });

    rx.blocking_recv().unwrap()
}

fn get_transaction(app: &mut App) -> Result<Value, serde_json::Error> {
    let (tx, mut rx) = mpsc::channel(1);
    let url = format!("https://api.helius.xyz/v0/transactions/?api-key={}", app.helius_api_key);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let client = reqwest::Client::new();
            let response = client.post(url)
                .json(&serde_json::json!({
                    "transactions": [app.search_sig]
                }))
                .send().await.unwrap();
            let data = response.text().await.unwrap();
            let json = serde_json::from_str(data.as_str());

            tx.send(json).await.unwrap();
        });

    rx.blocking_recv().unwrap()
}

fn lamports_to_sol(lamports: i64) -> f64 {
    lamports as f64 / 1000000000.0
}
