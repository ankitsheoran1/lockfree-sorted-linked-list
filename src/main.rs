use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

fn main() {
    println!("Hello, world!");
}

#[derive(Debug)]
struct Node<K, V> {
    key: Option<K>,
    val: AtomicPtr<V>,
    next: AtomicPtr<Node<K, V>>,
}

#[derive(Debug)]
struct LinkedList<K, V> {
    head: AtomicPtr<Node<K, V>>,
    tail: AtomicPtr<Node<K, V>>,
}


impl<K, V> Node<K, V> {
    fn empty() -> Self {
       Node { key: None,
        val: AtomicPtr::new(ptr::null_mut()),
        next: AtomicPtr::new(ptr::null_mut()),
    }
  }

  fn new(key: K, val: V) -> Self {
    Node {
        key: Some(key),
        val: AtomicPtr::new(Box::into_raw(Box::new(val))),
        next: AtomicPtr::new(ptr::null_mut()),
    }
  }
}


impl<K,V> Default for  LinkedList<K, V> {
  fn default() -> Self {
      let head = Box::new(Node::empty());
      let tail = Box::into_raw(Box::new(Node::empty()));
      head.next.store(tail, Ordering::SeqCst);

      LinkedList {
        head: AtomicPtr::new(Box::into_raw(head)),
        tail: AtomicPtr::new(tail),
      }
  }
}


impl<K, V> LinkedList<K, V> 
where 
K: Ord,
V: Copy, {

    fn delete(&self, key: &K, remove_nodes: &mut Vec<*mut Node<K, V>>) -> Option<V>  {
        let mut left_node = ptr::null_mut();
        let mut right_node;
        let mut right_node_next;
        loop {
           right_node = self.search(key, &mut left_node, remove_nodes);
           if right_node == self.tail.load(Ordering::SeqCst) || 
             unsafe { &*right_node }.
                   key
                   .as_ref()
                   .map(|k| k != key)
                   .unwrap_or(true) {
                    return None; 
                 }
            right_node_next = unsafe { &*right_node }.next.load(Ordering::SeqCst); 
            if !Self::is_marked_reference(right_node_next) 
               && unsafe { &*right_node }.next.compare_and_swap(right_node_next, Self::get_marked_reference(right_node_next), Ordering::SeqCst) == right_node_next 
               {
                break;
               }    
        }

        let node = unsafe { &*right_node };
        let old_val = unsafe { *node.val.load(Ordering::SeqCst) };
        if unsafe { &*left_node }
        .next
        .compare_and_swap(right_node, right_node_next, Ordering::SeqCst)
        != right_node
    {
        let _ = self.search(
            unsafe { &*right_node }.key.as_ref().unwrap(),
            &mut left_node,
            remove_nodes,
        );
    } else {
        remove_nodes.push(right_node);
    }

    Some(old_val)

   } 

   fn insert(self, key: K, val: V, remove_nodes: &mut Vec<*mut Node<K, V>>) -> Option<*mut V> {
       let mut new_node = Box::new(Node::new(key, val));
       let mut left_node = ptr::null_mut();

       loop {
         
         let right_node = self.search(new_node.key.as_ref().unwrap(), &mut left_node, remove_nodes);
         if right_node != self.tail.load(Ordering::SeqCst) &&
         unsafe { &*right_node }.key
                 .as_ref()
                 .map(|k| k == new_node.key.as_ref().unwrap())
                 .unwrap_or(false) {
                    let node = unsafe { &*right_node };
                    let value = Box::new(val);
                    let old = node.val.swap(Box::into_raw(value), Ordering::SeqCst);
                    remove_nodes.push(Box::into_raw(new_node));
                    return Some(old);

                 }

                 new_node.next.store(right_node, Ordering::SeqCst);
                 let mut new_node_ptr = Box::into_raw(new_node);
                 if unsafe { &*left_node }.next
                     .compare_and_swap(right_node, new_node_ptr, Ordering::SeqCst) == right_node {
                       return  None;
                     }
                     new_node = unsafe { Box::from_raw(new_node_ptr) };
            }
    }

    fn get(&self, key: &K,  remove_nodes: &mut Vec<*mut Node<K, V>>) -> Option<V> {
        let mut left_node = ptr::null_mut();
        let right_node = self.search(key, &mut left_node, remove_nodes);
        if right_node == self.tail.load(Ordering::SeqCst) 
           || unsafe { &*right_node }.key
              .as_ref()
              .map(|k| k != key )
              .unwrap_or(true) {
                return None;
              }

              unsafe { Some(*(&*right_node).val.load(Ordering::SeqCst)) }

    }

    fn is_marked_reference(ptr: *mut Node<K, V>) -> bool {
        (ptr as usize & 0x1) == 1
    }
    fn get_marked_reference(ptr: *mut Node<K, V>) -> *mut Node<K, V> {
        (ptr as usize | 0x1) as *mut _
    }
    fn get_unmarked_reference(ptr: *mut Node<K, V>) -> *mut Node<K, V> {
        (ptr as usize & !0x1) as *mut _
    }

    fn search(&self, search_key: &K, left_node: &mut *mut Node<K, V>, remove_nodes: &mut Vec<*mut Node<K, V>>) -> *mut Node<K, V> {
         let mut left_node_next = ptr::null_mut();
         let mut right_node;

         'search: loop {
            let mut t = self.head.load(Ordering::SeqCst);
            let mut t_next = unsafe {&*t}. next.load(Ordering::SeqCst);

            // Find left and right node 
            loop {
                if !Self::is_marked_reference(t_next) {
                    *left_node = t;
                    left_node_next = t_next;
                }
                // next iterate 
                if Self::is_marked_reference(t_next) {
                    t = Self::get_unmarked_reference(t_next);
                } else {
                    t = t_next;
                }

                if t == self.tail.load(Ordering::SeqCst) {
                    break;
                }
                t_next = unsafe { &*t }. next.load(Ordering::SeqCst);
                if !Self::is_marked_reference(t_next) && 
                 unsafe { &*t }.key
                         .as_ref()
                         .map( |k| k >= search_key)
                         .unwrap_or(false) {
                            break;
                         }

            }

            right_node = t;

            // if right and left nodes adjacent
            if left_node_next == right_node {
                if right_node != self.tail.load(Ordering::SeqCst) && 
                Self::is_marked_reference(unsafe { &*right_node }.next.load(Ordering::SeqCst)) {
                    continue 'search;
                } else {
                    return right_node;
                }

            }

            if unsafe { &**left_node }
                .next
                .compare_and_swap(left_node_next, right_node, Ordering::SeqCst)
                == left_node_next
            {
                let mut curr_node = left_node_next;

                loop {
                    assert_eq!(Self::is_marked_reference(curr_node), false);
                    remove_nodes.push(curr_node);
                    curr_node = unsafe { &*curr_node }.next.load(Ordering::SeqCst);
                    assert_eq!(Self::is_marked_reference(curr_node), true);
                    curr_node = Self::get_unmarked_reference(curr_node);
                    if curr_node == right_node {
                        break;
                    }
                }

                
                if right_node != self.tail.load(Ordering::SeqCst)    
                    && Self::is_marked_reference(unsafe { &*right_node }.next.load(OSC))
                {
                    continue 'search;
                } else {
                    return right_node;
                }


            }
        }



    }
}