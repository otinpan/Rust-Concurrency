#![feature(coroutines, coroutine_trait)]
#![feature(stmt_expr_attributes)]

use {
    std::{
        ops::{Coroutine,CoroutineState},
        thread,
        pin::Pin,
        sync::{Arc, Mutex},
        thread::{sleep},
        time::Duration,
        collections::{VecDeque},
        task::{RawWaker,RawWakerVTable,Waker,Context},
    },
};


// Pin: 自己参照を持つ型をmoveから守る
// Context: Wakerを渡すためのラッパー


trait SimpleFuture{
    type Output;
    fn poll(self:Pin<&mut Self>)->Poll<Self::Output>; //自己参照を持つ型をmoveから守る
}

#[derive(Debug)]
enum Poll<T>{
    Ready(T),
    Pending,
}

struct MyFuture{
    state: State,
    pinned: Pin<Box<dyn Coroutine<Yield=State,Return=State>>>,
}

#[derive(Debug)]
enum State{
    Start,
    Middle,
    End,
}

impl MyFuture{
    fn new()->Self{
        let coro=#[coroutine]||{
            println!("Start");
            yield State::Middle;
            println!("Middle");
            yield State::End;
            println!("End");
            return State::End;
        };
        Self{state: State::Start,pinned: Box::pin(coro)}       
    }
}

impl SimpleFuture for MyFuture{
    type Output=&'static str;
    fn poll(mut self:Pin<&mut Self>)->Poll<Self::Output>{
        let this=self.as_mut().get_mut();
        match this.pinned.as_mut().resume(()){
            CoroutineState::Yielded(val)=>{
                println!("Yielded: {:?}->{:?}",self.state,val);
                self.state=val;
                Poll::Pending
            }
            CoroutineState::Complete(val)=>{
                Poll::Ready("finished")
            }
        }
    }
}

fn main(){
    let mut my_fut=MyFuture::new();
    let mut pinned=Box::pin(my_fut);

    let mut poll_num=1;
    loop{
        println!("loop...");
        let res=pinned.as_mut().poll();
        match res{
            Poll::Ready(val)=>{
                println!("Cotroutine returned: {} poll={:?}",poll_num,res);
                break;
            }
            Poll::Pending=>{
                println!("Coroutine yielded: {} poll={:?}",poll_num,res);
            }
        }
        poll_num+=1;
        sleep(Duration::from_secs(2));
    }
}
