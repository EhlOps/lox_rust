use std::collections::HashMap;

pub struct Heap {
    bytes_allocated: usize,
    next_gc: usize,
    id_counter: usize,
    values: HashMap<usize, HeapVal>,
}

impl Heap {
    pub fn new() -> Heap {
        Heap {
            bytes_allocated: 0,
            next_gc: 1024 * 1024,
            id_counter: 0,
            values: HashMap::new(),
        }
    }

    pub fn allocate(&mut self, data: HeapData) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        self.values.insert(id, HeapVal { marked: false, data });
        id
    }

    pub fn free(&mut self, id: usize) {
        self.values.remove(&id);
    }

    pub fn get(&self, id: &usize) -> Option<&HeapData> {
        if let Some(val) = self.values.get(id) {
            Some(&val.data)
        } else {
            None
        }
    }

    pub fn get_all(&self) -> &HashMap<usize, HeapVal> {
        &self.values
    }

    pub fn mark(&mut self, id: usize) {
        if let Some(val) = self.values.get_mut(&id) {
            val.marked = true;
        }
    }
}

#[derive(Debug)]
pub struct HeapVal {
    marked: bool,
    data: HeapData
}

impl HeapVal {
    pub fn new(data: HeapData) -> HeapVal {
        HeapVal {
            marked: false,
            data
        }
    }

    pub fn is_marked(&self) -> bool {
        self.marked
    }
}

#[derive(Debug)]
pub enum HeapData {
    String(String)
}

impl HeapData {
    pub fn as_string(&self) -> &String {
        if let HeapData::String(s) = self {
            s
        } else {
            panic!("Expected string, got something else")
        }
    }
}