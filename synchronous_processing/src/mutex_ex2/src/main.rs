use std::sync::{Arc,Mutex};
use std::thread;
use std::time::Duration;
fn add_to_log_buffer(id:usize,log_buffer:Arc<Mutex<Vec::<String>>>){
    thread::sleep(Duration::from_millis(10*(id as u64 %3)));
    match log_buffer.try_lock(){
        Ok(mut buffer)=>{
            thread::sleep(Duration::from_millis(100));
            buffer.push(format!("Thread {} wrote to log",id));
        }
        Err(_)=>println!("Thread {} could not acquire lock",id),
    }
}
    

fn main(){
    let log_buffer=Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles=Vec::new();
    for i in 0..5{
        let log_buffer_i=log_buffer.clone();
        let handle=thread::spawn(move||{
           add_to_log_buffer(i,log_buffer_i); 
        });

        handles.push(handle);
    }

    for handle in handles{
        handle.join().unwrap();
    }

    let buffer=log_buffer.lock().unwrap();
    for v in buffer.iter(){
        println!("{}",v);
    }
}