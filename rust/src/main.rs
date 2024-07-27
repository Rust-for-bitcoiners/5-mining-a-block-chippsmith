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

//use bitcoin::secp256k1::hashes::{sha256, Hash};

fn test_weight() {
    let file = std::fs::File::open(
        "./mempool/0a331187bb44a28b342bd2fdfd2ff58147f0e4e43444b5efd89c71f3176caea6.json",
    )
    .expect("error reading files"); // Read file
    let json: Value = serde_json::from_reader(file).expect("error parsing json 1"); // Reads json from file
    let raw_tx = json["hex"].as_str().expect("error parsing json 2"); // Gets raw transaction hex from json
    let fee = json["fee"].as_u64().unwrap(); //Gets fee data from json
    let weight = json["weight"].as_u64().unwrap();
    let transaction_bytes = hex::decode(raw_tx).expect("error decoding hex"); // Decodes raw transaction data into a Vector of u8
    let mut byte_slice = transaction_bytes.as_slice(); // Turns Vec<u8> into slice to be read by Transaction::consensus_decode
    let transaction = Transaction::consensus_decode(&mut byte_slice).expect("error decoding"); // Turns raw transaction into struct Transaction
    let weight2 = transaction.total_size() as u64;
    println!("weight: {}, weight 2: {}", weight, weight2);
    assert_eq!(weight, weight2);
}
fn main() {
    // test_weight();
    // Read transactions from ./mempool
    let mut transactions = read_transactions_from_mempool_dir();
    println!("{:?}", transactions.len());
    let mut block_weight = 0;
    for tx in &transactions {
        block_weight += tx.transaction.weight().to_vbytes_ceil();
    }
    println!("mempool weight:  {:?} ", block_weight);

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

    println!("block hash: {:?}", block.block_hash());

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

    println!("{:?}", weight);
}
