use std::sync::{Arc,Mutex,Condvar};
use std::thread;
use std::collections::VecDeque;
use std::time::Duration;

fn produce(p:Arc<(Mutex<VecDeque<u32>>,Condvar,Condvar)>){
    let mut cnt=0;
    loop{
        let &(ref lock,ref is_full,ref is_empty  )=&*p;
        let mut buffer=lock.lock().unwrap();

        if cnt>=10{
            break;
        }
        //満杯だったらwaitする
        while buffer.len()>5{
            buffer=is_full.wait(buffer).unwrap();
        }
        buffer.push_back(cnt);
        is_empty.notify_all();
        println!("producer: {}",cnt);
        cnt+=1;
        thread::sleep(Duration::from_millis(100));

    }
}

fn consume(p:Arc<(Mutex<VecDeque<u32>>,Condvar,Condvar)>){
    let mut cnt=0;
    loop{
        let &(ref lock,ref is_full,ref is_empty  )=&*p;
        let mut buffer=lock.lock().unwrap();

        if cnt>=10{
            break;
        }
        if buffer.len()==0{
            buffer=is_empty.wait(buffer).unwrap();
        }

        let fr=match buffer.front(){
            Some(f)=>f,
            None=>panic!("the queue os empty"),
        };
        println!("consumer: {}",fr);
        buffer.pop_front();
        if buffer.len()<5{
            is_full.notify_all();
        }
        cnt+=1;
        thread::sleep(Duration::from_millis(100));
    }
}

fn main(){
    let buffer
    =Arc::new((Mutex::new(VecDeque::<u32>::new()),
    Condvar::new(),Condvar::new()));
    
    let lock_p=buffer.clone();
    let lock_c=buffer.clone();

    let producer=thread::spawn(move||{
        produce(lock_p);
    });

    let consumer=thread::spawn(move||{
        consume(lock_c)
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}