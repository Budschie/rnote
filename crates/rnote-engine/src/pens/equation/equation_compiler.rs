use std::{collections::HashMap, future::IntoFuture, rc::Rc, sync::Arc, thread};

use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    executor, FutureExt, StreamExt,
};

use crate::{
    engine::{EngineTask, EngineTaskSender},
    pens::pensconfig::equationconfig::EquationConfig,
    store::StrokeKey,
    strokes::EquationImage,
};

#[derive(Debug, Clone)]
pub struct CompilationTask {
    equation_code: String,
    equation_config: EquationConfig,
}

impl CompilationTask {
    pub fn new(equation: &EquationImage) -> CompilationTask {
        CompilationTask {
            equation_code: equation.equation_code.clone(),
            equation_config: equation.equation_config.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EquationCompilerTask {
    Compile(StrokeKey, CompilationTask),
    Quit,
}

#[derive(Debug)]
pub struct CompilationWorkResult {
    completed_work: Option<StrokeKey>,
}

#[derive(Debug, Clone)]
pub struct EquationCompilerTaskSender(mpsc::UnboundedSender<EquationCompilerTask>);

impl EquationCompilerTaskSender {
    pub fn send(&self, task: EquationCompilerTask) {
        if let Err(e) = self.0.unbounded_send(task) {
            let err = format!("{e:?}");
            tracing::error!(
                "Failed to send engine task {:?}, Err: {err}",
                e.into_inner()
            );
        }
    }
}

#[derive(Debug)]
pub struct EquationCompilerTaskReceiver(mpsc::UnboundedReceiver<EquationCompilerTask>);

impl EquationCompilerTaskReceiver {
    pub fn recv(&mut self) -> futures::stream::Next<'_, UnboundedReceiver<EquationCompilerTask>> {
        self.0.next()
    }
}

#[derive(Debug)]
pub struct EquationCompilerMainThread {
    pub tx: Option<EquationCompilerTaskSender>,
}

#[derive(Debug)]
pub struct EquationCompilerWorkerThread {
    rx: EquationCompilerTaskReceiver,
    engine_task_sender: EngineTaskSender,
    current_compilation_tasks: HashMap<StrokeKey, CompilationTask>,
}

impl EquationCompilerMainThread {
    pub fn new() -> Self {
        EquationCompilerMainThread { tx: None }
    }

    pub fn spawn_thread_and_run(&mut self, engine_task_sender: &EngineTaskSender) {
        let (tx, rx) = futures::channel::mpsc::unbounded::<EquationCompilerTask>();

        self.tx = Some(EquationCompilerTaskSender(tx));

        let engine_task_sender_long = engine_task_sender.clone();

        thread::spawn(move || {
            EquationCompilerWorkerThread::new(
                &engine_task_sender_long,
                EquationCompilerTaskReceiver(rx),
            )
            .run_internal();
        });
    }
}

impl EquationCompilerWorkerThread {
    pub fn new(
        engine_task_sender: &EngineTaskSender,
        rx: EquationCompilerTaskReceiver,
    ) -> EquationCompilerWorkerThread {
        EquationCompilerWorkerThread {
            rx,
            engine_task_sender: engine_task_sender.clone(),
            current_compilation_tasks: HashMap::default(),
        }
    }

    fn perform_compilation_work(&self) -> CompilationWorkResult {
        let item = self.current_compilation_tasks.iter().next();

        if let Some((key, task)) = item {
            let result = task.equation_config.generate_svg(&task.equation_code);

            match result {
                Ok(svg_code) => {
                    self.engine_task_sender.send(EngineTask::UpdateEquation {
                        key: key.clone(),
                        svg_code,
                    });
                    self.engine_task_sender.send(EngineTask::UpdateError {
                        key: key.clone(),
                        error: None,
                    });
                }
                Err(equation_error) => {
                    self.engine_task_sender.send(EngineTask::UpdateError {
                        key: key.clone(),
                        error: Some(equation_error),
                    });
                }
            }

            return CompilationWorkResult {
                completed_work: Some(key.clone()),
            };
        }

        CompilationWorkResult {
            completed_work: None,
        }
    }

    fn handle_message(&mut self, task: EquationCompilerTask) -> bool {
        match task {
            EquationCompilerTask::Compile(key, compilation_task) => {
                self.current_compilation_tasks.insert(key, compilation_task);
                false
            }
            EquationCompilerTask::Quit => true,
        }
    }

    fn receive_message_async(&mut self) -> Option<EquationCompilerTask> {
        self.rx.recv().now_or_never()?
    }

    fn receive_message_blocking(&mut self) -> Option<EquationCompilerTask> {
        executor::block_on(self.rx.recv().into_future())
    }

    fn receive_all_messages(&mut self, allow_block: bool) -> bool {
        let mut quit = false;

        // If one of the message receiving functions return an empty option, the compiler quits execution.
        if self.current_compilation_tasks.len() > 0 {
            // Loop until every message is handled
            while self.current_compilation_tasks.len() > 0 {
                if let Some(message) = self.receive_message_async() {
                    quit |= self.handle_message(message);
                } else {
                    return true;
                }
            }
        } else if allow_block {
            if let Some(message) = self.receive_message_blocking() {
                quit |= self.handle_message(message);
            } else {
                return true;
            }
        }

        quit
    }

    fn run_internal(&mut self) {
        loop {
            let result = self.perform_compilation_work();

            if let Some(stroke_key) = result.completed_work {
                self.current_compilation_tasks.remove(&stroke_key);
            }

            if self.receive_all_messages(self.current_compilation_tasks.len() <= 0) {
                break;
            }
        }
    }
}
