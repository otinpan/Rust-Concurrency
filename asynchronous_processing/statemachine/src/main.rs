use {
    std::{
        //ops::{Coroutine,CoroutineState},
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Waker},
        thread::{sleep},
        time::Duration,
    },
};


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
}

#[derive(Debug)]
enum State{
    Start,
    Middle,
    End,
}

impl MyFuture{
    fn new()->Self{
        Self{
            state:State::Start,
        }
    }
}

impl SimpleFuture for MyFuture{
    type Output=&'static str;
    fn poll(mut self:Pin<&mut Self>)->Poll<Self::Output>{
        let this=self.as_mut().get_mut();
        match this.state{
            State::Start=>{
                println!("Start");
                println!("Yielded: Start -> Middle");
                this.state=State::Middle;
                Poll::Pending
            }
            State::Middle=>{
                println!("Middle");
                println!("Yielded: Middle -> End");
                this.state=State::End;
                Poll::Pending
            }
            State::End=>{
                println!("End");
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


