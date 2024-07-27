use bitcoin::{
    absolute::LockTime,
    block::{Header, Version},
    consensus::{Decodable, Encodable},
    key::Secp256k1,
    secp256k1::SecretKey,
    Amount, Block, BlockHash, CompactTarget, OutPoint, PrivateKey, ScriptBuf, Sequence, Target,
    Transaction, TxIn, TxMerkleNode, TxOut, Txid, Weight, Witness,
};
use serde_json::{Result, Value};
use std::fs;

use bitcoin::transaction::Version as TransactionVersion;
use bitcoin_hashes::{sha256, sha256d::Hash as OtherHash};

use bitcoin::secp256k1::hashes::Hash;

use std::time::{SystemTime, UNIX_EPOCH};

// If time write signature verification for transactions

// Better way?
// Need fee from mempool to sort by fee rate and include best transactions for block
#[derive(Debug)]
pub struct TransactionWithDetails {
    pub transaction: Transaction,
    pub fee: u64,
}

// Calculate fee rate
impl TransactionWithDetails {
    fn fee_rate(&self) -> u64 {
        (self.fee) / self.transaction.weight().to_vbytes_ceil()
    }
}

pub fn read_transactions_from_mempool_dir() -> Vec<TransactionWithDetails> {
    let mut transactions = Vec::new(); // Empty vector to store transaction details
    let paths = fs::read_dir("./mempool").unwrap(); // Gets paths to all files in director ./mempool
    let mut total_weight = 0;
    // Loops through paths
    for path in paths {
        println!("{:?}", (path.as_ref()).unwrap().path());
        if path.as_ref().unwrap().path().as_mut_os_string() == "./mempool/mempool.json"{
            continue
        }
        let file = std::fs::File::open(path.unwrap().path()).expect("error reading files"); // Read file
        let json: Value = serde_json::from_reader(file).expect("error parsing json 1"); // Reads json from file
        let raw_tx = json["hex"].as_str().expect("error parsing json 2"); // Gets raw transaction hex from json
        let fee = json["fee"].as_u64().unwrap(); //Gets fee data from json
        let weight = json["weight"].as_u64().unwrap();
        total_weight += weight;
        let transaction_bytes = hex::decode(raw_tx).expect("error decoding hex"); // Decodes raw transaction data into a Vector of u8
        let mut byte_slice = transaction_bytes.as_slice(); // Turns Vec<u8> into slice to be read by Transaction::consensus_decode
        let transaction = Transaction::consensus_decode(&mut byte_slice).expect("error decoding"); // Turns raw transaction into struct Transaction
        let transaction_with_details = TransactionWithDetails { transaction, fee }; //must be a better way to add the fee than creating a new struct
        transactions.push(transaction_with_details);
    }
    println!("total_weight:  {:?}", total_weight);
    transactions
}

// sort transactions highest fee rate first
pub fn sort_by_fee_rate(transactions: &mut Vec<TransactionWithDetails>) -> () {
    transactions.sort_by(|a, b| {
        b.fee_rate()
            .partial_cmp(&a.fee_rate())
            .expect("error comparing fee rates")
    })
}

// Removes lowest fee rate transactions so block stays within 4 MB size
pub fn cull_transactions(transactions: Vec<TransactionWithDetails>) -> Vec<TransactionWithDetails> {
    let mut v: Vec<TransactionWithDetails> = Vec::new();
    let mut total_weight = 0;
    for tx in transactions {
        total_weight += tx.transaction.weight().to_vbytes_ceil();
        v.push(tx);
        if total_weight > 1_000_000 {
            break;
        }
    }
    v
}

// There must be an easier way to deal with adding the fee field to the already existing Transaction struct
pub fn drop_fee_from_transaction_with_details(
    transactions: Vec<TransactionWithDetails>,
) -> Vec<Transaction> {
    let mut v = Vec::new();
    for tx in transactions {
        v.push(tx.transaction)
    }
    v
}

// Sort transactions by weight
pub fn sort_by_weight(transactions: &mut Vec<Transaction>) -> () {
    //function to sort by weight highest first
    transactions.sort_by(|a, b| {
        b.weight()
            .partial_cmp(&a.weight())
            .expect("error comparing weights")
    });
}

// Make Default block header
pub fn block_header() -> Header {
    Header {
        version: Version::NO_SOFT_FORK_SIGNALLING, // Version enum with no signaling
        prev_blockhash: BlockHash::from_raw_hash(*OtherHash::from_bytes_ref(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ])), // Prev blockhash can be anything as long as less than target per readme
        merkle_root: TxMerkleNode::from_raw_hash(*OtherHash::from_bytes_ref(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ])), // Default merkle root wiil be changed when it can be calulated from the list of txs
        time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32,                          // would be current time in practice
        bits: Target::from_hex(
            &"0x0000ffff00000000000000000000000000000000000000000000000000000000",
        )
        .expect("error creating Target")
        .to_compact_lossy(), //creates bits from target given in readme
        nonce: 0,                                  // To be incremented to find valid block
    }
}

// Gets recieve script from dummy slice data to use in recieving coinbase output
fn get_recieve_script() -> ScriptBuf {
    //throw away data for private key to create recieve script
    let secp = Secp256k1::new();
    let random_slice = [
        100, 201, 90, 100, 62, 2, 0, 1, 245, 16, 61, 34, 125, 68, 218, 78, 208, 212, 237, 141, 235,
        243, 23, 194, 49, 250, 30, 91, 66, 190, 195, 82,
    ]
    .as_slice();

    let sk = SecretKey::from_slice(random_slice).expect("32 bytes, within curve order");
    let pk = PrivateKey {
        compressed: true,
        network: bitcoin::NetworkKind::Main,
        inner: sk,
    };
    let pub_k = pk.public_key(&secp);
    ScriptBuf::new_p2pk(&pub_k) //
}

// Creates Coinbase transaction with data about what block and who mined it in the coinbase input script_sig
pub fn create_coinbase_transaction() -> Transaction {
    let mut script_sig = String::from("853900 Btc_Chris").into_bytes(); //block mined in and who mined it.
    script_sig.insert(0, 16_u8); // prepend length of script
    let script_sig = ScriptBuf::from(script_sig);
    let witness = Witness::from(vec![[0_u8; 32].to_vec()]);

    let input = TxIn {
        previous_output: OutPoint {
            //txid must be 32 bytes of zeros for coinbase input and vout must be max value 0xffffffff
            txid: Txid::from_raw_hash(*OtherHash::from_bytes_ref(&[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ])),
            vout: 0xffffffff,
        },
        script_sig: script_sig,
        sequence: Sequence(0xffffffff),
        witness: witness, // Need 32 byte reserved value all zeros if block contains segwit transactions
    };

    /*
    cant use because need the first output of the coinbas transaction to be the wTXID commitment
    let op_return_data = String::from("Hello Rust Class").into_bytes();
    // seems excessive but couldnt satisfy pushbytes with any other type
    let op_return_data: &[u8; 16] = op_return_data.as_slice().try_into().expect("error");
    let op_return_script = ScriptBuf::new_op_return(op_return_data);
     */

    let output = TxOut {
        value: Amount::ZERO,
        script_pubkey: ScriptBuf::new(),
    };

    let value = Amount::from_btc(3.125).expect("error creating amount for output");
    let recieve_script = get_recieve_script(); // need to create to recieve script

    let output2 = TxOut {
        value: value,
        script_pubkey: recieve_script,
    };

    Transaction {
        version: TransactionVersion::ONE,
        lock_time: LockTime::ZERO,
        input: vec![input],
        output: vec![output, output2],
    }
}

// Adds coinbase transaction and transactions to make a block template and updates merkle_root(Returned Block.header is ready to be mined)
pub fn block_template(mut txdata: Vec<Transaction>) -> Block {
    let coinbase_tx = create_coinbase_transaction();
    txdata.insert(0, coinbase_tx);

    let header = block_header();
    let mut block = Block { header, txdata };

    let merkle_root = block
        .compute_merkle_root()
        .expect("error calculating merkle root ");
    block.header.merkle_root = merkle_root;

    //TODO:  Add valid witness commitment to output.scipt_pub_key of coinbase output

    /*
    let witness_root = block.witness_root().expect("error calculating root");
    let witness_commitment = Block::compute_witness_commitment(&witness_root, &[0]);

    block.txdata[0].output[0].script_pubkey =
        ScriptBuf::from_bytes(witness_commitment.as_byte_array().to_vec());
     */
    

    block
}


// Increments and updates nonce until valid block is found
pub fn mine_block(block: &mut Block) -> () {
    loop {
        
        let hash = block.header.block_hash();
        let hash = hash.as_byte_array();
        
        // Last two bytes must be zero because of endianess
        if hash[30] == 0 && hash[31] == 0 {
            break;
            
        }
        block.header.nonce += 1;
        
    }

}

#[cfg(test)]
mod tests {
    use bitcoin::Target;

    use super::*;

    #[test]
    fn test_sort_by_fee_rate() {
        let mut transactions = read_transactions_from_mempool_dir();
        sort_by_fee_rate(&mut transactions);
        println!("{:?}", transactions)
    }

    #[test]
    fn test_coinbase() {
        let coinbase_tx = create_coinbase_transaction();
        println!("{:?}", coinbase_tx);
    }

    #[test]

    fn test_compact_target() {
        let u8: u8 = 0xff;
        let u16: u16 = 0xffff;

        let bits =
            Target::from_hex(&"0x0000ffff00000000000000000000000000000000000000000000000000000000")
                .expect("error creating Target")
                .to_compact_lossy();
        println!("{:?}", bits)
    }

    #[test]

    fn test_weight() {
        let file = std::fs::File::open(
            "mempool/00000a2d1a9e29116b539b85b6e893213b1ed95a08b7526a8d59a4b088fc6571.json",
        )
        .expect("error reading files"); // Read file
        let json: Value = serde_json::from_reader(file).expect("error parsing json 1"); // Reads json from file
        let raw_tx = json["hex"].as_str().expect("error parsing json 2"); // Gets raw transaction hex from json
        let fee = json["fee"].as_u64().unwrap(); //Gets fee data from json
        let weight = json["weight"].as_u64().unwrap();
        let transaction_bytes = hex::decode(raw_tx).expect("error decoding hex"); // Decodes raw transaction data into a Vector of u8
        let mut byte_slice = transaction_bytes.as_slice(); // Turns Vec<u8> into slice to be read by Transaction::consensus_decode
        let transaction = Transaction::consensus_decode(&mut byte_slice).expect("error decoding"); // Turns raw transaction into struct Transaction
        let weight2 = transaction.weight().to_vbytes_ceil();
        assert_eq!(weight, weight2);
    }


    #[test]

    fn header_hash(){
        let tx = create_coinbase_transaction();
        let mut block = block_template(vec!(tx));

        mine_block(&mut block);

        println!("aa {:?}", block.block_hash())

    }
}
