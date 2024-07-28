mod utils;
use bitcoin::consensus::{Decodable, Encodable};
use bitcoin::Transaction;
use serde_json::Value;
use utils::{
    block_template, cull_transactions, drop_fee_from_transaction_with_details, mine_block,
    read_transactions_from_mempool_dir, sort_by_fee_rate,
};

use std::fs::File;
use std::io::prelude::*;


fn main() {
    // test_weight();
    // Read transactions from ./mempool
    let mut transactions = read_transactions_from_mempool_dir();
    let mut block_weight = 0;
    for tx in &transactions {
        block_weight += tx.transaction.weight().to_vbytes_ceil();
    }

    // Sort transactions by fee rate
    sort_by_fee_rate(&mut transactions);

    // Make sure block is under 4mb limit
    let transactions = cull_transactions(transactions);

    // Remove fee from TranactionWithDetails struct so we can put it in the block... Better way?
    let transactions = drop_fee_from_transaction_with_details(transactions);

    // Create block template
    let mut block = block_template(transactions);

    // Mine block (adds one to nonce until finds a nonce that yields sha256 hash of block header less than difficulty and updates block.header.nonce )
    mine_block(&mut block); // Nonce 104874 yields valid block


    // Creates file out.txt in root
    let mut file = File::create("out.txt").expect("error opening file");

    // Encodes header for writing to file
    let mut header = Vec::new(); //Vector for storing encoded header
    block
        .header
        .consensus_encode(&mut header)
        .expect("error encoding block");

    // Writes header on first line of file
    write!(file, "{}\n", hex::encode(header)).expect("error writing header");

    let mut v = Vec::new(); // Vector for storing encoded coinbase transaction

    // Encode Coinbase Transaction for writing to file
    let coinbase_transaction = &block.txdata[0];
    coinbase_transaction
        .consensus_encode(&mut v)
        .expect("error encoding transaction");

    // Writes coinbase transaction to second line of file
    write!(file, "{}\n", hex::encode(v)).expect("error writing coinbase transaction");

    // Loops through all transactions in block and writes them to individual lines in file
    let mut weight = 0;
    for tx in &block.txdata {
        weight += tx.weight().to_vbytes_ceil();
        write!(file, "{}\n", tx.compute_txid()).expect("error writing txid");
    }

}
