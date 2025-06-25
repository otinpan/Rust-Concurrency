use std::sync::{Arc, Mutex}; 
use std::thread;

fn some_func(lock: Arc<Mutex<u64>>) { //スレッド用関数
    loop {
        // ロックしないとMutex型の中の値は参照不可
        let mut val = lock.lock().unwrap(); // lock関数でロックを獲得
        *val += 1;
        println!("{}", *val);
    }
}

fn main() {
    // Arcはスレッドセーフな参照カウンタ型のスマートポインタ
    let lock0 = Arc::new(Mutex::new(0)); //ミューテックス用変数を所持する

    // 参照カウンタがインクリメントされるのみで
    // 中身はクローンされない
    let lock1 = lock0.clone(); //クローンする。参照カウンタがインクリメントされる

    // スレッド生成
    // クロージャ内変数へmove
    let th0 = thread::spawn(move || { //所有権移動
        some_func(lock0);
    });

    // スレッド生成
    // クロージャ内変数へmove
    let th1 = thread::spawn(move || {
        some_func(lock1);
    });

    // 待ち合わせ
    th0.join().unwrap();
    th1.join().unwrap();
}