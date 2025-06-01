Here is your original memo translated into English and formatted in Markdown:

---

# Span Logging System Design

## Overview

* Logs are written when a span is entered or exited.
* Each thread holds many spans (LocalSpans).
* Periodically, each thread transfers its LocalSpans to a global structure.

## Structure

```text
|rawSpan|rawSpan|rawSpan|  rawSpan|rawSpan|rawSpan  |

|       span queue      |         span queue        |

|       span line       |         span line         |

|        LocalSpanStack                             |
```

### Components

#### LocalSpanStack

* Created when a thread is launched.
* Exists for the entire lifetime of the thread.

#### SpanLine

* Created via `set_local_parent`, and associated with a `LocalParentGuard`.

  * Registered using `register_span_line`.
* When `LocalParentGuard` is dropped, `span_line.collect` returns the rawSpans.

#### SpanQueue

* Created via `LocalSpan::enter_with_local_parent`, etc.

  * The target SpanLine is determined at this point (captured via `current_span_line`).

* `SpanLine::start_span` is called.

* Then, `SpanQueue::start_span` is called.

* `SpanQueue` maintains an internal counter that increments with each `start_span`.

* When a `LocalSpan` is dropped, its end\_time is recorded.

* When `LocalParentGuard` is dropped, the entire SpanQueue is taken.

* When logs are emitted, they are added to the SpanQueue via the LocalSpanStack.

  * Thread-local information determines which SpanQueue is currently in use.

* The terminal SpanQueue uses an object pool to:

  * Avoid malloc.
  * Enable effective caching.

* When `LocalParentGuard` is dropped, `collect_spans_and_token` is triggered.

---

## Questions

(Empty section – fill in as needed)

---

# `fast` Design

* Based on sync trace.

  * Events and spans are **not** moved to other threads.
* No parent-child relationship.

  * No need for `SpanLine`, `SpanQueue`, etc.
* Uses a simple `Span` type only.

  * `start_time` at creation.
  * `end_time` at drop.
* Like `fastrace`, uses thread-local storage.
* Aggressively uses object storage:

  * Improves locality.
  * Reduces allocator calls.
* Span format design:

  * Minimize dynamic fields.
  * Use enums for type differentiation.

---

## TODOs

* **Log collection from TLS**:

  * Should each thread push logs?
  * Or should a reporter thread pull them?
* **Flush on thread interruption**:

  * TLS makes flushing difficult? → Can be handled via `Drop` implementation.
  * But `Drop` call order depends on thread shutdown order.

    * If `main` or consumer shuts down first, data loss may occur.

---

# Possibility of Custom Arena

### Features

* Provides thread-specific memory regions.

  * Similar to thread-local storage, but allows access to other threads' regions.
* Assigns a unique key to each thread to determine which memory region it can access.
* Hard limit on how much data a thread can hold.
* No limit on the number of threads.

### Challenges

* How to initialize memory when a new thread is created?
* How to generate non-colliding keys?

---

# Summary

* Use thread-local storage in worker threads.
* During runtime shutdown:

  * All worker threads are shut down.
* After worker shutdown:

  * Consumer thread (persists even after shutdown) synchronously flushes remaining logs.

----

* spanにenterしたりexitすると, ログを書き込む
* それぞれのスレッドがspanをたくさん持つ (LocalSpans)
* 定期的にそのスレッドが, LocalSpansを, globalに転送する
### 構造

```text
|rawSpan|rawSpan|rawSpan|  rawSpan|rawSpan|rawSpan  |
|       span queue      |         span queue        |
|       span line       |         span line         |
|        LocalSpanStack                             |
```

* LocalSpanStack
  * threadが起動された瞬間に作られる
  * そのスレッドが生きている限りずっとある
* SpanLine
  * set_local_parent により生成される, LocalParentGuard と紐づいてる
    * register_span_line で登録される
  * LocalParentGuard が drop されると, span_line.collect によってrawSpansが返される
* SpanQueue
  * LocalSpan::enter_with_local_parent 等で生成される
    * ここで, 入るべきSpanLineが決定的に決まることに注意(current_span_lineで撮ってる)
  * SpanLineのstart_spanが呼ばれる
  * そのまま, SpqnQueueの start_spanが呼ばれる
  * SpqnQueueは内部にカウンタを持っていて,start_span を呼ぶたびにincrementされる
  * LocalSpanがdropされると, end_timeが記録される
  * LocalParentGuard がdropされると, SpanQueueごとtakeされる
* ログを吐くと, LocalSpanStackを経由して最終的にspan queueni入れられる
  * thread localで, 今何番目のspan queueを使っているか, などんの情報がわかる
* 末端のspan queueは, object poolを使って, mallocが起こらないように && キャッシュが効くように
* LocalParentGuard が drop されると, `collect_spans_and_token` によって
 ### 疑問
*  
# fastのデザイン
* sync traceがベース
  * イベントやスパンは別のスレッドにmoveしない
* 親子関係は持たない
  * spanline, spanqueue等の区別がいらない
* 単純な Span 型のみを持つ
  * 生成時にstart_time
  * drop時に end_time
* fastraceのように, thread local storageを使う
* fastraceのように, object storageは積極的に活用する
  * locality, allocator呼び出しの低減
* spanのformatを決める
  * 動的fieldを極力減らす
  * enumでtype分けするイメージ
# TODO
* tlsに溜まったログを集めるタイミング
  * それぞれのスレッドがpushすべきか, reporter threadがpullしに行くか
* threadが中断された際のflushの処理
  * TLSだと, flushが難しい可能性がある -> いや, dropの実装で対応できる
  * ただ, drop呼び出しはthread終了の順番に依存する模様 (main or consumerが先に終了しちゃうとロスが発生)
# Arena 自作の可能性
### 機能
* スレッド固有のメモリ領域を提供する
  * thread local storageに近いが, 他のthreadのmemory regionにもアクセスができる
* threadに固有のキーを割り当て, そのキーによってそのスレッドがどのメモリリージョンを触れるのかを判断
* 1つのスレッドが持てるデータ量はハードリミットが決まっている。
* しかし, 登録できるスレッドの数は制限しない。
### 課題
* 新しくthreadが作成された時, どうやってその
* 衝突しないようなキーの作成方法
# まとめ
* worker threadにthread local storageを置く
* runtimeのshutdownの時に, workerは全てshutdownされる
* worker threadが消えた後に, consumer thread(こいつはshutdownされても残り続ける!)がflushを同期的に行う
