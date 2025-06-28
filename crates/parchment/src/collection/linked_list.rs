//! High-performance doubly-linked list implementation for blot management.
//!
//! This module provides a custom doubly-linked list optimized for Parchment's
//! document operations. Unlike standard collections, this implementation uses
//! raw pointers for maximum performance while maintaining memory safety through
//! careful ownership management.
//!
//! # Design Goals
//!
//! - **Performance**: O(1) insertion/deletion at head and tail
//! - **Memory Safety**: Safe abstractions over unsafe pointer operations
//! - **Flexibility**: Support for insertion/deletion at arbitrary positions
//! - **Iterator Support**: Standard Rust iteration patterns
//!
//! # Usage Examples
//!
//! ```rust
//! use parchment::collection::LinkedList;
//!
//! // Create and populate a list
//! let mut list = LinkedList::new();
//! list.push(1);
//! list.push(2);
//! list.insert(1, 10);
//!
//! // Access elements
//! assert_eq!(list.get(0), Some(&1));
//! assert_eq!(list.get(1), Some(&10));
//! assert_eq!(list.get(2), Some(&2));
//!
//! // Search and iterate
//! let found = list.find(|&x| x == 10);
//! assert_eq!(found, Some(&10));
//!
//! for item in list.iter() {
//!     // Process each item
//! }
//! ```

use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A node in the doubly-linked list.
///
/// Each node contains a value and optional pointers to the next and previous
/// nodes in the list. The `next` field is public to allow external manipulation
/// while `prev` is private to maintain list integrity.
pub struct Node<T> {
    /// The value stored in this node
    pub val: T,
    /// Pointer to the next node in the list (public for external access)
    pub next: Option<NonNull<Node<T>>>,
    /// Pointer to the previous node in the list (private for integrity)
    prev: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    /// Create a new node with the given value.
    ///
    /// The node is created with no connections to other nodes.
    fn new(t: T) -> Node<T> {
        Node {
            val: t,
            prev: None,
            next: None,
        }
    }
}

/// A high-performance doubly-linked list implementation.
///
/// This linked list is optimized for document operations where frequent
/// insertion and deletion at arbitrary positions is required. It uses
/// raw pointers for performance while maintaining memory safety through
/// careful ownership management.
///
/// # Characteristics
///
/// - **Doubly-linked**: Each node has pointers to both next and previous nodes
/// - **Null-pointer optimized**: Uses `NonNull<T>` for better performance
/// - **Memory safe**: Proper cleanup through `Drop` implementation
/// - **Generic**: Works with any type `T`
///
/// # Memory Layout
///
/// ```text
/// head -> [Node] <-> [Node] <-> [Node] <- tail
///           |          |          |
///         val_1      val_2      val_3
/// ```
pub struct LinkedList<T> {
    /// Number of elements in the list
    pub length: u32,
    /// Pointer to the first node (None if empty)
    pub head: Option<NonNull<Node<T>>>,
    /// Pointer to the last node (None if empty)
    pub tail: Option<NonNull<Node<T>>>,
    /// Phantom data to indicate ownership of boxed nodes
    marker: PhantomData<Box<Node<T>>>,
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedList<T> {
    /// Create a new empty linked list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let list: LinkedList<i32> = LinkedList::new();
    /// assert_eq!(list.length, 0);
    /// ```
    pub fn new() -> Self {
        Self {
            length: 0,
            head: None,
            tail: None,
            marker: PhantomData,
        }
    }

    /// Insert an element at the head (beginning) of the list.
    ///
    /// This operation is O(1) and updates the head pointer to point to the new node.
    ///
    /// # Arguments
    ///
    /// * `obj` - The value to insert
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.insert_at_head(1);
    /// list.insert_at_head(2);
    /// assert_eq!(list.get(0), Some(&2)); // Most recently inserted
    /// assert_eq!(list.get(1), Some(&1));
    /// ```
    pub fn insert_at_head(&mut self, obj: T) {
        let mut node = Box::new(Node::new(obj));
        node.next = self.head;
        node.prev = None;
        let node_ptr = NonNull::new(Box::into_raw(node));
        match self.head {
            None => self.tail = node_ptr,
            Some(head_ptr) => unsafe { (*head_ptr.as_ptr()).prev = node_ptr },
        }
        self.head = node_ptr;
        self.length += 1;
    }

    /// Insert an element at the tail (end) of the list.
    ///
    /// This operation is O(1) and updates the tail pointer to point to the new node.
    ///
    /// # Arguments
    ///
    /// * `obj` - The value to insert
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.insert_at_tail(1);
    /// list.insert_at_tail(2);
    /// assert_eq!(list.get(0), Some(&1));
    /// assert_eq!(list.get(1), Some(&2)); // Most recently inserted
    /// ```
    pub fn insert_at_tail(&mut self, obj: T) {
        let mut node = Box::new(Node::new(obj));
        node.next = None;
        node.prev = self.tail;
        let node_ptr = NonNull::new(Box::into_raw(node));
        match self.tail {
            None => self.head = node_ptr,
            Some(tail_ptr) => unsafe { (*tail_ptr.as_ptr()).next = node_ptr },
        }
        self.tail = node_ptr;
        self.length += 1;
    }

    /// Insert an element at the specified index.
    ///
    /// This operation is O(n) as it requires traversal to the insertion point.
    /// If the index equals the length, the element is inserted at the tail.
    /// If the index is 0, the element is inserted at the head.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to insert at (0-based)
    /// * `obj` - The value to insert
    ///
    /// # Panics
    ///
    /// Panics if `index > length`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.insert_at_ith(0, 1);
    /// list.insert_at_ith(1, 3);
    /// list.insert_at_ith(1, 2); // Insert in middle
    /// assert_eq!(list.get(1), Some(&2));
    /// ```
    pub fn insert_at_ith(&mut self, index: u32, obj: T) {
        if self.length < index {
            panic!("Index out of bounds");
        }

        if index == 0 || self.head.is_none() {
            self.insert_at_head(obj);
            return;
        }

        if self.length == index {
            self.insert_at_tail(obj);
            return;
        }

        if let Some(mut ith_node) = self.head {
            for _ in 0..index {
                unsafe {
                    match (*ith_node.as_ptr()).next {
                        None => panic!("Index out of bounds"),
                        Some(next_ptr) => ith_node = next_ptr,
                    }
                }
            }

            let mut node = Box::new(Node::new(obj));
            unsafe {
                node.prev = (*ith_node.as_ptr()).prev;
                node.next = Some(ith_node);
                if let Some(p) = (*ith_node.as_ptr()).prev {
                    let node_ptr = NonNull::new(Box::into_raw(node));
                    println!("{:?}", (*p.as_ptr()).next);
                    (*p.as_ptr()).next = node_ptr;
                    (*ith_node.as_ptr()).prev = node_ptr;
                    self.length += 1;
                }
            }
        }
    }

    /// Remove and return the element at the head of the list.
    ///
    /// This operation is O(1) and updates the head pointer to the next node.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The value that was at the head
    /// * `None` - If the list is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// assert_eq!(list.delete_head(), Some(1));
    /// assert_eq!(list.length, 1);
    /// ```
    pub fn delete_head(&mut self) -> Option<T> {
        // Safety: head_ptr points to a leaked boxed node managed by this list
        // We reassign pointers that pointed to the head node
        if self.length == 0 {
            return None;
        }

        self.head.map(|head_ptr| unsafe {
            let old_head = Box::from_raw(head_ptr.as_ptr());
            match old_head.next {
                Some(mut next_ptr) => next_ptr.as_mut().prev = None,
                None => self.tail = None,
            }
            self.head = old_head.next;
            self.length = self.length.checked_add_signed(-1).unwrap_or(0);
            old_head.val
        })
        // None
    }

    /// Remove and return the element at the tail of the list.
    ///
    /// This operation is O(1) and updates the tail pointer to the previous node.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The value that was at the tail
    /// * `None` - If the list is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// assert_eq!(list.delete_tail(), Some(2));
    /// assert_eq!(list.length, 1);
    /// ```
    pub fn delete_tail(&mut self) -> Option<T> {
        // Safety: tail_ptr points to a leaked boxed node managed by this list
        // We reassign pointers that pointed to the tail node
        self.tail.map(|tail_ptr| unsafe {
            let old_tail = Box::from_raw(tail_ptr.as_ptr());
            match old_tail.prev {
                Some(mut prev) => prev.as_mut().next = None,
                None => self.head = None,
            }
            self.tail = old_tail.prev;
            self.length -= 1;
            old_tail.val
        })
    }

    /// Remove and return the element at the specified index.
    ///
    /// This operation is O(n) as it requires traversal to the deletion point.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to delete from (0-based)
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The value that was at the index
    /// * `None` - If the index is out of bounds
    ///
    /// # Panics
    ///
    /// Panics if `index >= length`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// list.push(3);
    /// assert_eq!(list.delete_ith(1), Some(2));
    /// assert_eq!(list.length, 2);
    /// ```
    pub fn delete_ith(&mut self, index: u32) -> Option<T> {
        if self.length <= index {
            panic!("Index out of bounds");
        }

        if index == 0 || self.head.is_none() {
            return self.delete_head();
        }

        if self.length - 1 == index {
            return self.delete_tail();
        }

        if let Some(mut ith_node) = self.head {
            for _ in 0..index {
                unsafe {
                    match (*ith_node.as_ptr()).next {
                        None => panic!("Index out of bounds"),
                        Some(next_ptr) => ith_node = next_ptr,
                    }
                }
            }

            unsafe {
                let old_ith = Box::from_raw(ith_node.as_ptr());
                if let Some(mut prev) = old_ith.prev {
                    prev.as_mut().next = old_ith.next;
                }
                if let Some(mut next) = old_ith.next {
                    next.as_mut().prev = old_ith.prev;
                }

                self.length -= 1;
                Some(old_ith.val)
            }
        } else {
            None
        }
    }

    /// Get a reference to the element at the specified index.
    ///
    /// This operation is O(n) as it requires traversal to the target position.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to access (0-based)
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - Reference to the value at the index
    /// * `None` - If the index is out of bounds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// assert_eq!(list.get(0), Some(&1));
    /// assert_eq!(list.get(5), None);
    /// ```
    pub fn get(&self, index: i32) -> Option<&T> {
        Self::get_ith_node(self.head, index).map(|ptr| unsafe { &(*ptr.as_ptr()).val })
    }

    /// Get a mutable reference to the element at the specified index.
    ///
    /// This operation is O(n) as it requires traversal to the target position.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to access (0-based)
    ///
    /// # Returns
    ///
    /// * `Some(&mut T)` - Mutable reference to the value at the index
    /// * `None` - If the index is out of bounds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// if let Some(val) = list.get_mut(0) {
    ///     *val = 10;
    /// }
    /// assert_eq!(list.get(0), Some(&10));
    /// ```
    pub fn get_mut(&mut self, index: i32) -> Option<&mut T> {
        Self::get_ith_node(self.head, index).map(|ptr| unsafe { &mut (*ptr.as_ptr()).val })
    }

    /// Internal helper to get the node at a specific index.
    ///
    /// This is a recursive function that traverses the list to find the node
    /// at the specified index.
    ///
    /// # Arguments
    ///
    /// * `node` - Starting node for traversal
    /// * `index` - Target index (decremented with each recursive call)
    ///
    /// # Returns
    ///
    /// * `Some(NonNull<Node<T>>)` - Pointer to the node at the index
    /// * `None` - If the index is out of bounds
    fn get_ith_node(node: Option<NonNull<Node<T>>>, index: i32) -> Option<NonNull<Node<T>>> {
        match node {
            None => None,
            Some(next_ptr) => match index {
                0 => Some(next_ptr),
                _ => Self::get_ith_node(unsafe { (*next_ptr.as_ptr()).next }, index - 1),
            },
        }
    }

    /// Find the first element that matches the predicate.
    ///
    /// This operation is O(n) in the worst case, traversing the list until
    /// a matching element is found or the end is reached.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function that returns true for the desired element
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - Reference to the first matching element
    /// * `None` - If no element matches the predicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// list.push(3);
    ///
    /// let found = list.find(|&x| x > 2);
    /// assert_eq!(found, Some(&3));
    /// ```
    pub fn find<F>(&self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        let mut current_node = self.head;
        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                if predicate(&node_ref.val) {
                    return Some(&node_ref.val);
                }
                current_node = node_ref.next;
            }
        }
        None
    }

    /// Find the index of the first element that matches the predicate.
    ///
    /// This operation is O(n) in the worst case, traversing the list until
    /// a matching element is found or the end is reached.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function that returns true for the desired element
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - Index of the first matching element
    /// * `None` - If no element matches the predicate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// list.push(3);
    ///
    /// let index = list.index_of(|&x| x == 2);
    /// assert_eq!(index, Some(1));
    /// ```
    pub fn index_of<F>(&self, predicate: F) -> Option<usize>
    where
        F: Fn(&T) -> bool,
    {
        let mut current_node = self.head;
        let mut index = 0;
        while let Some(node_ptr) = current_node {
            unsafe {
                let node_ref = node_ptr.as_ref();
                if predicate(&node_ref.val) {
                    return Some(index);
                }
                current_node = node_ref.next;
                index += 1;
            }
        }
        None
    }

    /// Calculate offset to a given index.
    ///
    /// For a linked list, the offset is equivalent to the index since
    /// each element is at a sequential position.
    ///
    /// # Arguments
    ///
    /// * `index` - The target index
    ///
    /// # Returns
    ///
    /// The offset value (same as index for linked lists)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let list: LinkedList<i32> = LinkedList::new();
    /// assert_eq!(list.offset(5), 5);
    /// ```
    pub fn offset(&self, index: usize) -> usize {
        // For LinkedList, offset is just the index itself
        index
    }

    /// Check if the list contains an element matching the predicate.
    ///
    /// This is a convenience method that returns true if any element
    /// matches the given predicate.
    ///
    /// # Arguments
    ///
    /// * `predicate` - Function that returns true for the desired element
    ///
    /// # Returns
    ///
    /// * `true` - If at least one element matches
    /// * `false` - If no elements match
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    ///
    /// assert!(list.contains(|&x| x == 2));
    /// assert!(!list.contains(|&x| x == 5));
    /// ```
    pub fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        self.find(predicate).is_some()
    }

    /// Iterate over elements in a specific range with a callback.
    ///
    /// This method calls the provided callback for each element in the range
    /// [start, start+length), passing both the element and its index.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting index (inclusive)
    /// * `length` - Number of elements to process
    /// * `callback` - Function called for each element with (element, index)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// list.push(3);
    /// list.push(4);
    ///
    /// let mut sum = 0;
    /// list.for_each_at(1, 2, |&val, _idx| sum += val);
    /// assert_eq!(sum, 5); // 2 + 3
    /// ```
    pub fn for_each_at<F>(&self, start: usize, length: usize, mut callback: F)
    where
        F: FnMut(&T, usize),
    {
        let mut current_node = self.head;
        let mut current_index = 0;

        // Skip to start position
        while current_index < start && current_node.is_some() {
            unsafe {
                current_node = current_node.unwrap().as_ref().next;
            }
            current_index += 1;
        }

        // Execute callback for length items
        let mut processed = 0;
        while processed < length && current_node.is_some() {
            unsafe {
                let node_ref = current_node.unwrap().as_ref();
                callback(&node_ref.val, current_index);
                current_node = node_ref.next;
            }
            current_index += 1;
            processed += 1;
        }
    }

    /// Push an element to the end of the list.
    ///
    /// This is a convenience method that calls [`insert_at_tail`].
    ///
    /// # Arguments
    ///
    /// * `obj` - The value to push
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// assert_eq!(list.length, 2);
    /// ```
    ///
    /// [`insert_at_tail`]: #method.insert_at_tail
    pub fn push(&mut self, obj: T) {
        self.insert_at_tail(obj);
    }

    /// Pop an element from the end of the list.
    ///
    /// This is a convenience method that calls [`delete_tail`].
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The value that was at the end
    /// * `None` - If the list is empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// assert_eq!(list.pop(), Some(1));
    /// assert_eq!(list.pop(), None);
    /// ```
    ///
    /// [`delete_tail`]: #method.delete_tail
    pub fn pop(&mut self) -> Option<T> {
        self.delete_tail()
    }

    /// Insert an element at the specified index.
    ///
    /// This is a convenience method that calls [`insert_at_ith`] with
    /// proper bounds checking.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to insert at
    /// * `obj` - The value to insert
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.insert(0, 1);
    /// list.insert(1, 2);
    /// assert_eq!(list.get(1), Some(&2));
    /// ```
    ///
    /// [`insert_at_ith`]: #method.insert_at_ith
    pub fn insert(&mut self, index: i32, obj: T) {
        self.insert_at_ith(index as u32, obj);
    }

    /// Remove an element at the specified index.
    ///
    /// This is a convenience method that calls [`delete_ith`] with
    /// bounds checking to prevent panics.
    ///
    /// # Arguments
    ///
    /// * `index` - The position to remove from
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The value that was removed
    /// * `None` - If the index is out of bounds
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// assert_eq!(list.remove(0), Some(1));
    /// assert_eq!(list.remove(10), None);
    /// ```
    ///
    /// [`delete_ith`]: #method.delete_ith
    pub fn remove(&mut self, index: i32) -> Option<T> {
        if index >= 0 && index < self.length as i32 {
            self.delete_ith(index as u32)
        } else {
            None
        }
    }

    /// Create an iterator over the linked list.
    ///
    /// The iterator yields raw pointers to the values for performance reasons.
    /// This is safe as long as the list is not modified during iteration.
    ///
    /// # Returns
    ///
    /// A [`LinkedListIterator`] that yields `*const T` pointers
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    ///
    /// for item_ptr in list.iter() {
    ///     unsafe {
    ///         println!("Value: {}", *item_ptr);
    ///     }
    /// }
    /// ```
    ///
    /// [`LinkedListIterator`]: struct.LinkedListIterator.html
    pub fn iter(&self) -> LinkedListIterator<T> {
        LinkedListIterator {
            current: self.head,
            phantom: std::marker::PhantomData,
        }
    }
}

/// Iterator for traversing a [`LinkedList`].
///
/// This iterator yields raw pointers to the values for performance reasons.
/// The pointers are valid as long as the list is not modified during iteration.
///
/// # Safety
///
/// The iterator returns `*const T` pointers that must be dereferenced safely.
/// The caller must ensure the list is not modified during iteration.
///
/// [`LinkedList`]: struct.LinkedList.html
pub struct LinkedListIterator<T> {
    /// Current node being processed
    current: Option<NonNull<Node<T>>>,
    /// Phantom data for type safety
    phantom: std::marker::PhantomData<T>,
}

impl<T> Iterator for LinkedListIterator<T> {
    type Item = *const T;

    /// Advance the iterator and return the next item.
    ///
    /// # Returns
    ///
    /// * `Some(*const T)` - Pointer to the next value
    /// * `None` - If the iterator is exhausted
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_ptr) = self.current {
            unsafe {
                let node = current_ptr.as_ref();
                self.current = node.next;
                Some(&node.val as *const T)
            }
        } else {
            None
        }
    }
}

impl<T> Drop for LinkedList<T> {
    /// Clean up the linked list by deallocating all nodes.
    ///
    /// This implementation ensures that all heap-allocated nodes are
    /// properly freed when the list goes out of scope.
    fn drop(&mut self) {
        // Pop items until there are none left
        while self.delete_head().is_some() {}
    }
}

impl<T> Display for LinkedList<T>
where
    T: Display,
{
    /// Format the linked list for display.
    ///
    /// This implementation shows all elements in the list separated by commas.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use parchment::collection::LinkedList;
    ///
    /// let mut list = LinkedList::new();
    /// list.push(1);
    /// list.push(2);
    /// list.push(3);
    /// println!("{}", list); // Outputs: "1, 2, 3"
    /// ```
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.head {
            Some(node) => write!(f, "{}", unsafe { node.as_ref() }),
            None => Ok(()),
        }
    }
}

impl<T> Display for Node<T>
where
    T: Display,
{
    /// Format a node and its successors for display.
    ///
    /// This implementation recursively formats the current node and all
    /// following nodes, creating a comma-separated list.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.next {
            Some(node) => write!(f, "{}, {}", self.val, unsafe { node.as_ref() }),
            None => write!(f, "{}", self.val),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::LinkedList;

    #[test]
    fn insert_at_tail_works() {
        let mut list = LinkedList::<i32>::new();
        let second_value = 2;
        list.insert_at_tail(1);
        list.insert_at_tail(second_value);
        println!("Linked List is {list}");
        match list.get(1) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 1"),
        }
    }
    #[test]
    fn insert_at_head_works() {
        let mut list = LinkedList::<i32>::new();
        let second_value = 2;
        list.insert_at_head(1);
        list.insert_at_head(second_value);
        println!("Linked List is {list}");
        match list.get(0) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 0"),
        }
    }

    #[test]
    fn insert_at_ith_can_add_to_tail() {
        let mut list = LinkedList::<i32>::new();
        let second_value = 2;
        list.insert_at_ith(0, 0);
        list.insert_at_ith(1, second_value);
        println!("Linked List is {list}");
        match list.get(1) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 1"),
        }
    }

    #[test]
    fn insert_at_ith_can_add_to_head() {
        let mut list = LinkedList::<i32>::new();
        let second_value = 2;
        list.insert_at_ith(0, 1);
        list.insert_at_ith(0, second_value);
        println!("Linked List is {list}");
        match list.get(0) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 0"),
        }
    }

    #[test]
    fn insert_at_ith_can_add_to_middle() {
        let mut list = LinkedList::<i32>::new();
        let second_value = 2;
        let third_value = 3;
        list.insert_at_ith(0, 1);
        list.insert_at_ith(1, second_value);
        list.insert_at_ith(1, third_value);
        println!("Linked List is {list}");
        match list.get(1) {
            Some(val) => assert_eq!(*val, third_value),
            None => panic!("Expected to find {third_value} at index 1"),
        }

        match list.get(2) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 1"),
        }
    }

    #[test]
    fn insert_at_ith_and_delete_at_ith_in_the_middle() {
        // Insert and delete in the middle of the list to ensure pointers are updated correctly
        let mut list = LinkedList::<i32>::new();
        let first_value = 0;
        let second_value = 1;
        let third_value = 2;
        let fourth_value = 3;

        list.insert_at_ith(0, first_value);
        list.insert_at_ith(1, fourth_value);
        list.insert_at_ith(1, third_value);
        list.insert_at_ith(1, second_value);

        list.delete_ith(2);
        list.insert_at_ith(2, third_value);

        for (i, expected) in [
            (0, first_value),
            (1, second_value),
            (2, third_value),
            (3, fourth_value),
        ] {
            match list.get(i) {
                Some(val) => assert_eq!(*val, expected),
                None => panic!("Expected to find {expected} at index {i}"),
            }
        }
    }

    #[test]
    fn insert_at_ith_and_delete_ith_work_over_many_iterations() {
        let mut list = LinkedList::<i32>::new();
        for i in 0..100 {
            list.insert_at_ith(i, i.try_into().unwrap());
        }

        // Pop even numbers to 50
        for i in 0..50 {
            println!("list.length {}", list.length);
            if i % 2 == 0 {
                list.delete_ith(i);
            }
        }

        assert_eq!(list.length, 75);

        // Insert even numbers back
        for i in 0..50 {
            if i % 2 == 0 {
                list.insert_at_ith(i, i.try_into().unwrap());
            }
        }

        assert_eq!(list.length, 100);

        // Ensure numbers were adderd back and we're able to traverse nodes
        if let Some(val) = list.get(78) {
            assert_eq!(*val, 78);
        } else {
            panic!("Expected to find 78 at index 78");
        }
    }

    #[test]
    fn delete_tail_works() {
        let mut list = LinkedList::<i32>::new();
        let first_value = 1;
        let second_value = 2;
        list.insert_at_tail(first_value);
        list.insert_at_tail(second_value);
        match list.delete_tail() {
            Some(val) => assert_eq!(val, 2),
            None => panic!("Expected to remove {second_value} at tail"),
        }

        println!("Linked List is {list}");
        match list.get(0) {
            Some(val) => assert_eq!(*val, first_value),
            None => panic!("Expected to find {first_value} at index 0"),
        }
    }

    #[test]
    fn delete_head_works() {
        let mut list = LinkedList::<i32>::new();
        let first_value = 1;
        let second_value = 2;
        list.insert_at_tail(first_value);
        list.insert_at_tail(second_value);
        match list.delete_head() {
            Some(val) => assert_eq!(val, 1),
            None => panic!("Expected to remove {first_value} at head"),
        }

        println!("Linked List is {list}");
        match list.get(0) {
            Some(val) => assert_eq!(*val, second_value),
            None => panic!("Expected to find {second_value} at index 0"),
        }
    }

    #[test]
    fn delete_ith_can_delete_at_tail() {
        let mut list = LinkedList::<i32>::new();
        let first_value = 1;
        let second_value = 2;
        list.insert_at_tail(first_value);
        list.insert_at_tail(second_value);
        match list.delete_ith(1) {
            Some(val) => assert_eq!(val, 2),
            None => panic!("Expected to remove {second_value} at tail"),
        }

        assert_eq!(list.length, 1);
    }

    #[test]
    fn delete_ith_can_delete_at_head() {
        let mut list = LinkedList::<i32>::new();
        let first_value = 1;
        let second_value = 2;
        list.insert_at_tail(first_value);
        list.insert_at_tail(second_value);
        match list.delete_ith(0) {
            Some(val) => assert_eq!(val, 1),
            None => panic!("Expected to remove {first_value} at tail"),
        }

        assert_eq!(list.length, 1);
    }

    #[test]
    fn delete_ith_can_delete_in_middle() {
        let mut list = LinkedList::<i32>::new();
        let first_value = 1;
        let second_value = 2;
        let third_value = 3;
        list.insert_at_tail(first_value);
        list.insert_at_tail(second_value);
        list.insert_at_tail(third_value);
        match list.delete_ith(1) {
            Some(val) => assert_eq!(val, 2),
            None => panic!("Expected to remove {second_value} at tail"),
        }

        match list.get(1) {
            Some(val) => assert_eq!(*val, third_value),
            None => panic!("Expected to find {third_value} at index 1"),
        }
    }

    #[test]
    fn create_numeric_list() {
        let mut list = LinkedList::<i32>::new();
        list.insert_at_tail(1);
        list.insert_at_tail(2);
        list.insert_at_tail(3);
        println!("Linked List is {list}");
        assert_eq!(3, list.length);
    }

    #[test]
    fn create_string_list() {
        let mut list_str = LinkedList::<String>::new();
        list_str.insert_at_tail("A".to_string());
        list_str.insert_at_tail("B".to_string());
        list_str.insert_at_tail("C".to_string());
        println!("Linked List is {list_str}");
        assert_eq!(3, list_str.length);
    }

    #[test]
    fn get_by_index_in_numeric_list() {
        let mut list = LinkedList::<i32>::new();
        list.insert_at_tail(1);
        list.insert_at_tail(2);
        println!("Linked List is {list}");
        let retrieved_item = list.get(1);
        assert!(retrieved_item.is_some());
        assert_eq!(2, *retrieved_item.unwrap());
    }

    #[test]
    fn get_by_index_in_string_list() {
        let mut list_str = LinkedList::<String>::new();
        list_str.insert_at_tail("A".to_string());
        list_str.insert_at_tail("B".to_string());
        println!("Linked List is {list_str}");
        let retrieved_item = list_str.get(1);
        assert!(retrieved_item.is_some());
        assert_eq!("B", *retrieved_item.unwrap());
    }

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn delete_ith_panics_if_index_equals_length() {
        let mut list = LinkedList::<i32>::new();
        list.insert_at_tail(1);
        list.insert_at_tail(2);
        // length is 2, so index 2 is out of bounds
        list.delete_ith(2);
    }
}
