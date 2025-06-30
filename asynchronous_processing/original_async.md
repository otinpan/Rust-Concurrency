# 自作 `async/await`

### ステートマシン
```rust
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

    fn spawn(&self, future: impl SimpleFuture<Output = &'static str> + Send + 'static) {
        let task=Arc::new(Task {
            future: Mutex::new(Some(Box::pin(future))),
            executor: self.inner.clone(), //共通のExecutorの参照カウンタを+1
         });
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

// main /////////////////////////////////////////////////////////////////////////////
fn main(){
    let executor=Executor::new();

    executor.spawn(MyFuture::new());

    executor.run()
}
```

```rust
pub struct Context<'a>{
    waker: &'a Waker,
}
impl<'a> Context<'a>{
    pub fn waker(&self)->&Waker{
        self.waker
    }
}
```
```
Start
Yielded: Start -> Middle
Middle
Yielded: Middle -> End
End
```



Contextの中身にはWakerの参照が入っている。
```rust
Task::poll
let waker=create_waker(task.clone())
```
でTaskのアドレスが埋め込まれているWakerを作る。
```rust
let mut cx=Context::from_waker(&waker);
```
でコンテキストを作る。コンテキストはTaskの参照が入っている。
```rust
future.poll(&mut cx);
```
futureを`poll`するときにコンテキストを渡す。 
`poll()`では
```rust
cx.waker().wake_by_ref();
```
でコンテキストからWakerを取り出し、`wake_by_ref`を呼ぶことでWakerに埋め込まれたタスクをExecutorにpushする。  

## コルーチンバージョン
```rust
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
```
Executor、Task、Wakerは据え置きでコンパイルする。
```
error[E0277]: `(dyn Coroutine<Return = State, Yield = State> + 'static)` cannot be sent between threads safely
    --> src/main.rs:192:33
     |
192  |         future: Mutex::new(Some(Box::pin(my_future))),
     |                                 ^^^^^^^^^^^^^^^^^^^ `(dyn Coroutine<Return = State, Yield = State> + 'static)` cannot be sent between threads safely
     |
     = help: the trait `Send` is not implemented for `(dyn Coroutine<Return = State, Yield = State> + 'static)`
     = note: required for `Unique<(dyn Coroutine<Return = State, Yield = State> + 'static)>` to implement `Send`
```

`dyn Coroutine`は`Send`を実装していないから、スレッド間で送れない。

```rust
struct Task {
    future: Mutex<Option<Pin<Box<dyn SimpleFuture<Output = &'static str> + Send>>>>,
    ...
}
```
Taskのなかに入るFutureがSendであることを要求していた。ただ、`MyFuture`はこのようになっている。
```rust
struct MyFuture {
    state: State,
    pinned: Pin<Box<dyn Coroutine<Yield = State, Return = State>>>,
}
```
`dyn Coroutine`はSendではない。

### Sendについて
Sendとは「値をスレッド間で安全に移動できることを保障するトレイト」　　
スレッドは
* スタックをそれぞれが持つ
* ヒープを共有する  

Rustのcoroutineは
* 独自のスタックを内部に持つ
* スタックはスレッドに紐づく
* 自己参照を持つ  

コルーチン関数はスレッド間を安全に移動できない。他のスレッドに移動させると参照が壊れるからSendを実装できない。

### 解決策
マルチスレッドはあきらめる。
```rust
struct Task {
    future: Mutex<Option<Pin<Box<dyn SimpleFuture<Output = &'static str>>>>>,
    executor: Arc<Executor>,
}
```


* シングルスレッド × 1 コア
    - プログラムも 1 スレッドだけ。
    - CPU も 1 つしか作業できない。
    - 完全に 1 つの処理しか同時に進まない。

* シングルスレッド × 複数コア
    - プログラム自体は 1 本のスレッドだけ動かす。
    - でも CPU は複数コアがある。
    - 結局そのプログラムは 1 コアしか使わない（他のコアは別プロセスや OS の処理に使われる）。

* マルチスレッド × 1 コア
    - プログラムが複数のスレッドを作る。
    - でも CPU が 1 コアしかない。
    - CPU は物理的に同時に 1 スレッドしか実行できない。
    - 代わりに OS が高速に 切り替え（コンテキストスイッチ） する。


* マルチスレッド × 複数コア
    - 複数のスレッドを同時に動かすプログラム。
    - OS がスレッドを複数コアに分散させて処理する。


ステートマシンなら`spawn`してマルチスレッド × 1 コアが書ける  
コルーチンだとシングルスレッド × 1 コアになる (Sendが実装できないから)

