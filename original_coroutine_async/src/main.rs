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
        rc::Rc,
    },
};


// Pin: 自己参照を持つ型をmoveから守る
// Context: Wakerを渡すためのラッパー


trait SimpleFuture{
    type Output;
    fn poll(self:Pin<&mut Self>,cx: &mut Context)->Poll<Self::Output>; //自己参照を持つ型をmoveから守る
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
            return State::End;
        };
        Self{state: State::Start,pinned: Box::pin(coro)}       
    }
}

impl SimpleFuture for MyFuture{
    type Output=&'static str;
    fn poll(mut self:Pin<&mut Self>,cx:&mut Context)->Poll<Self::Output>{
        let this=self.as_mut().get_mut();
        match this.pinned.as_mut().resume(()){
            CoroutineState::Yielded(val)=>{
                println!("Yielded: {:?}->{:?}",self.state,val);
                self.state=val;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            CoroutineState::Complete(val)=>{
                Poll::Ready("finished")
            }
        }
    }
}

// Task ///////////////////////////////////////////////////////////
struct Task {
    future: Mutex<Option<Pin<Box<dyn SimpleFuture<Output = &'static str>>>>>,
    executor: Arc<Executor>,
}

impl Task{
    fn schedule(self: &Arc<Self>){
        unsafe{
            (*self.executor).queue.lock().unwrap().push_back(self.clone());
        }
    }
    fn poll(self: Arc<Self>){
        let mut fut_slot=self.future.lock().unwrap();
        if let Some(mut fut)=fut_slot.take(){
            let waker=create_waker(self.clone());
            let mut ctx=Context::from_waker(&waker);

            let res=fut.as_mut().poll(&mut ctx);
            match res{
                Poll::Ready(val)=>{

                }
                Poll::Pending=>{
                    *fut_slot=Some(fut)
                }
            }
        }
    }
}

// Executor //////////////////////////////////////////////////////
struct Executor{
    queue: Mutex<VecDeque<Arc<Task>>>,
}



impl Executor{
    fn new()->Self{
        Self { 
            queue:  Mutex::new(VecDeque::new()),
        }
    }

    fn spawn(&self, task:Arc<Task>){
        self.queue.lock().unwrap().push_back(task);
    }

    fn run(&self){
        loop{
            let task_opt=self.queue.lock().unwrap().pop_front();

            if let Some(task)=task_opt{
                task.poll();
            }else{
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}


// Waker ///////////////////////////////////////////////////////////
fn create_waker(task: Arc<Task>) -> Waker{
    // Wakerのクローンをつくる
    unsafe fn clone(data: *const ()) -> RawWaker {
        let arc = Arc::from_raw(data as *const Task); //from_rawしたarcはdropすると参照カウントが-1になる
        let arc_clone = arc.clone();
        std::mem::forget(arc); //ドロップした後に参照カウンタを-1しない
        RawWaker::new(data, &VTABLE)
    }

    // Taskを再スケジューリングしてWakerを消費する
    unsafe fn wake(data: *const()){
        let task=Arc::from_raw(data as *const Task);
        task.schedule();
    }

    // Wakerは消費されない
    unsafe fn wake_by_ref(data: *const()){
        let task=Arc::from_raw(data as *const Task);
        task.schedule();
        std::mem::forget(task);
    }

    //参照カウントを1減らす
    unsafe fn drop(data: *const()){
        let _=Arc::from_raw(data as *const Task);
    }

    //clone,wake,wake_by_ref,dropがWakerに紐づく
    static VTABLE: RawWakerVTable=RawWakerVTable::new(clone,wake,wake_by_ref,drop);

    let ptr=Arc::into_raw(task) as *const(); //Arc<Task>を生ポインタ化
    let raw=RawWaker::new(ptr,&VTABLE); //RawWakerを作成
    unsafe {Waker::from_raw(raw)} //Waker::from_rawで安全なWakerに変換
}


// main /////////////////////////////////////////////////////////////////////////////////
fn main() {
    let mut executor = Arc::new(Executor::new());
    let my_future = MyFuture::new();

    let task = Arc::new(Task {
        future: Mutex::new(Some(Box::pin(my_future))),
        executor: executor.clone() 
    });


    executor.spawn(task);
    executor.run();
}