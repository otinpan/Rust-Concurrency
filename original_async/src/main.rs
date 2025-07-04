use {
    std::{
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
    fn poll(self:Pin<&mut Self>,cx: &mut Context)->Poll<Self::Output>; 
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
    fn poll(mut self:Pin<&mut Self>,cx:&mut Context)->Poll<Self::Output>{
        let this=self.as_mut().get_mut();
        match this.state{
            State::Start=>{
                println!("Start");
                println!("Yielded: Start -> Middle");
                this.state=State::Middle;
                cx.waker().wake_by_ref(); //自分自身をqueueにpushする
                Poll::Pending
            }
            State::Middle=>{
                println!("Middle");
                println!("Yielded: Middle -> End");
                this.state=State::End;
                cx.waker().wake_by_ref(); 
                Poll::Pending
            }
            State::End=>{
                println!("End");
                Poll::Ready("finished")
            }

        }
    }
}



// Task /////////////////////////////////////////////////////////////////////////////////////////
//SimpleFutureトレイトを実装していて、出力が&`static strであるような型をヒープに保存
//Sendによって他スレッドに送ることが出来る
type BoxFuture = Box<dyn SimpleFuture<Output = &'static str> + Send>;
struct Task{
    future: Mutex<Option<Pin<BoxFuture>>>,
    executor: Arc<ExecutorInner>,
}

impl Task{
    //自分自身をExecutorのqueueにpushする
    fn schedule(self: &Arc<Self>){
        self.executor.queue.lock().unwrap().push_back(self.clone());
    }

    fn poll(self: Arc<Self>){
        let mut fut_slot=self.future.lock().unwrap();
        if let Some(mut fut)=fut_slot.take(){ //takeするとtask.futureの中身はNoneになる
            let waker=create_waker(self.clone()); //WakerにはTaskのアドレスが埋め込まれている
            let mut ctx=Context::from_waker(&waker); //Contextの中にWakerが入っている

            let res=fut.as_mut().poll(&mut ctx); //pollを呼び出すときにContextを渡す
            match res{
                Poll::Ready(val)=>{
                    
                }
                Poll::Pending=>{
                    *fut_slot=Some(fut); //Noneになったtask.futureの中身を戻す
                }
           }
        }
    }
}

// Executor ///////////////////////////////////////////////////////////////////////////
//ExecutorInnerは実行待ちのタスクを管理する
//複数スレッドからタスクが追加、取り出しされないようにする
struct ExecutorInner{
    queue: Mutex<VecDeque<Arc<Task>>>, //同じタスクを共有
}

//同じキューを共有
struct Executor{
    inner: Arc<ExecutorInner>,
}

impl Executor{
    fn new()->Self{
        Self {
            inner: Arc::new(ExecutorInner {
                queue: Mutex::new(VecDeque::new()),
             })
        }
    }

    fn spawn(&self, task:Arc<Task>) {
        self.inner.queue.lock().unwrap().push_back(task);
    }



    fn run(&self){
        loop{
            let task_opt=self.inner.queue.lock().unwrap().pop_front();

            if let Some(task)=task_opt{
                task.poll();
            }else{
                thread::sleep(Duration::from_millis(10));
            }
        }
       
    }
}

// Waker ///////////////////////////////////////////////////////////////////////////////////
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


// main ///////////////////////////////////////////////////////////////////////////////////////////////////
fn main(){
    let executor=Executor::new();
    let my_future=MyFuture::new();
    let task=Arc::new(Task{
        future: Mutex::new(Some(Box::pin(my_future))),
        executor: executor.inner.clone(),
    });

    executor.spawn(task);
    executor.run()
}