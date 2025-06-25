use std::sync::{Arc,Mutex};
use std::thread;
use rand::Rng;
use std::time::Duration;

fn add_random(id:&str,lock:Arc<Mutex<u64>>){
    loop{
        let mut v=lock.lock().unwrap();
        let num=rand::thread_rng().gen_range(1..6);

        if *v+num>=100{
            println!("thread{}: {}",id,100);
            break;
        }
        *v+=num;

        println!("thread{}: {}",id,v);
        drop(v);
        thread::sleep(Duration::from_millis(100));
    }
}

fn main(){
    let lock=Arc::new(Mutex::new(0));
    let lock0=lock.clone();
    let lock1=lock.clone();

    let th0=thread::spawn(move||{
       add_random("A",lock0); 
    });

    let th1=thread::spawn(move||{
        add_random("B",lock1);
    });

    th0.join();
    th1.join();

}