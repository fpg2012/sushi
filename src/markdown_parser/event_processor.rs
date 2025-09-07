use pulldown_cmark::Event;

pub trait EventProcessor {
    fn apply<'a>(
        &'a mut self,
        iter: impl Iterator<Item = Event<'a>>,
    ) -> impl Iterator<Item = Event<'a>>;
}

pub trait ProcessWith {
    fn process_with<'a, P>(self, processor: &'a mut P) -> impl Iterator<Item = Event<'a>>
    where
        P: EventProcessor,
        Self: Sized + Iterator<Item = Event<'a>>;
}

impl<I> ProcessWith for I
where
    I: Iterator,
{
    fn process_with<'a, P>(self, processor: &'a mut P) -> impl Iterator<Item = Event<'a>>
    where
        P: EventProcessor,
        I: Iterator<Item = Event<'a>> + Sized,
    {
        processor.apply(self)
    }
}

pub struct DummyEventProcessor;

impl EventProcessor for DummyEventProcessor {
    fn apply<'a>(
        &'a mut self,
        iter: impl Iterator<Item = Event<'a>>,
    ) -> impl Iterator<Item = Event<'a>> {
        iter
    }
}

pub struct DebugEventProcessor {}

impl EventProcessor for DebugEventProcessor {
    fn apply<'a>(
        &'a mut self,
        iter: impl Iterator<Item = Event<'a>>,
    ) -> impl Iterator<Item = Event<'a>> {
        iter.map(|event| {
            println!("[event] {:?}", event);
            event
        })
    }
}
