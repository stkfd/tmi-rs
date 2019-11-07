(function() {var implementors = {};
implementors["futures_channel"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"futures_channel/oneshot/struct.Receiver.html\" title=\"struct futures_channel::oneshot::Receiver\">Receiver</a>&lt;T&gt;",synthetic:false,types:["futures_channel::oneshot::Receiver"]},];
implementors["tokio_executor"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"tokio_executor/blocking/struct.Blocking.html\" title=\"struct tokio_executor::blocking::Blocking\">Blocking</a>&lt;T&gt;",synthetic:false,types:["tokio_executor::blocking::Blocking"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"tokio_executor/threadpool/struct.Shutdown.html\" title=\"struct tokio_executor::threadpool::Shutdown\">Shutdown</a>",synthetic:false,types:["tokio_executor::threadpool::shutdown::Shutdown"]},];
implementors["tokio_sync"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"tokio_sync/oneshot/struct.Receiver.html\" title=\"struct tokio_sync::oneshot::Receiver\">Receiver</a>&lt;T&gt;",synthetic:false,types:["tokio_sync::oneshot::Receiver"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        })()