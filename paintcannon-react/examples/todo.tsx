import React, {useCallback, useEffect, useRef, useState} from 'react';
import type {
  CSSStyleProperties,
  PaintChangeEvent,
  PaintKeyboardEvent,
  PaintScrollEvent,
} from 'paintcannon';
import {
  Button,
  Div,
  Form,
  Input,
  Span,
  render,
  useApp,
  type DivElement,
  type InputElement,
} from '../src/index.ts';

interface Todo {
  id: number;
  text: string;
  completed: boolean;
}

interface TodoScrollMetrics {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
}

type HoverTarget =
  | 'add'
  | `check:${number}`
  | `edit:${number}`
  | `delete:${number}`;

function TodoApp(): React.ReactElement {
  const {exit, paintCannon} = useApp();
  const listRef = useRef<DivElement | null>(null);
  const mainInputRef = useRef<InputElement | null>(null);
  const editReturnSelectionIdRef = useRef<number | undefined>(undefined);
  const [draft, setDraft] = useState('');
  const [todos, setTodos] = useState<Todo[]>([
    {id: 1, text: 'Ship the React reconciler', completed: true},
    {id: 2, text: 'Write the first pass of the docs', completed: false},
    {id: 3, text: 'Add screenshots to the README', completed: false},
    {id: 4, text: 'Sketch the next paintcannon-react hook API', completed: false},
    {id: 5, text: 'Support <a> tags with clickable terminal links', completed: false},
    {id: 6, text: 'Support clickable labels for inputs', completed: false},
  ]);
  const [nextId, setNextId] = useState(7);
  const [editingId, setEditingId] = useState<number | undefined>();
  const [editingText, setEditingText] = useState('');
  const [selectedIndex, setSelectedIndex] = useState<number | undefined>();
  const [mainInputFocused, setMainInputFocused] = useState(false);
  const [hovered, setHovered] = useState<HoverTarget | undefined>();
  const [scrollMetrics, setScrollMetrics] = useState<TodoScrollMetrics>({
    scrollTop: 0,
    scrollHeight: 0,
    clientHeight: 0,
  });

  const updateScrollMetrics = useCallback((metrics: TodoScrollMetrics): void => {
    setScrollMetrics(current => (
      current.scrollTop === metrics.scrollTop &&
      current.scrollHeight === metrics.scrollHeight &&
      current.clientHeight === metrics.clientHeight
        ? current
        : metrics
    ));
  }, []);

  const readListScrollMetrics = useCallback((): void => {
    const list = listRef.current;
    if (list === null) {
      return;
    }

    updateScrollMetrics({
      scrollTop: list.scrollTop,
      scrollHeight: list.scrollHeight,
      clientHeight: list.clientHeight,
    });
  }, [updateScrollMetrics]);

  useEffect(() => {
    readListScrollMetrics();
  }, [todos, editingId, editingText, readListScrollMetrics]);

  useEffect(() => {
    if (selectedIndex === undefined) {
      return;
    }

    if (todos.length === 0) {
      setSelectedIndex(undefined);
      return;
    }

    if (selectedIndex >= todos.length) {
      setSelectedIndex(todos.length - 1);
    }
  }, [selectedIndex, todos.length]);

  useEffect(() => {
    if (selectedIndex === undefined) {
      return;
    }

    const list = listRef.current;
    if (list === null) {
      return;
    }

    const rowTop = selectedIndex * todoRowStride;
    const rowBottom = rowTop + todoRowHeight;
    const visibleTop = list.scrollTop;
    const visibleBottom = visibleTop + list.clientHeight;

    if (rowTop < visibleTop) {
      list.scrollTop = rowTop;
    } else if (rowBottom > visibleBottom) {
      list.scrollTop = Math.max(0, rowBottom - list.clientHeight);
    }

    readListScrollMetrics();
  }, [readListScrollMetrics, selectedIndex]);

  useEffect(() => {
    const handleResize = (): void => readListScrollMetrics();
    paintCannon.addEventListener('resize', handleResize);
    return () => paintCannon.removeEventListener('resize', handleResize);
  }, [paintCannon, readListScrollMetrics]);

  const addTodo = (): void => {
    const text = draft.trim();
    if (text.length === 0) {
      return;
    }

    setTodos(current => [{id: nextId, text, completed: false}, ...current]);
    setNextId(id => id + 1);
    setDraft('');
  };

  const beginEdit = (todo: Todo, returnToSelection = false): void => {
    editReturnSelectionIdRef.current = returnToSelection ? todo.id : undefined;
    setSelectedIndex(undefined);
    setEditingId(todo.id);
    setEditingText(todo.text);
  };

  const focusMainInputAfterCommit = (): void => {
    queueMicrotask(() => {
      mainInputRef.current?.focus();
    });
  };

  const restoreSelectionAfterEdit = (id: number): boolean => {
    const index = todos.findIndex(todo => todo.id === id);
    if (index === -1) {
      return false;
    }

    mainInputRef.current?.blur();
    setSelectedIndex(index);
    return true;
  };

  const commitEdit = (): void => {
    if (editingId === undefined) {
      return;
    }

    const editedId = editingId;
    const returnSelectionId = editReturnSelectionIdRef.current;
    editReturnSelectionIdRef.current = undefined;
    const text = editingText.trim();
    if (text.length === 0) {
      setTodos(current => current.filter(todo => todo.id !== editedId));
    } else {
      setTodos(current => current.map(todo => (
        todo.id === editedId ? {...todo, text} : todo
      )));
    }
    setEditingId(undefined);
    setEditingText('');
    if (text.length > 0 && returnSelectionId === editedId && restoreSelectionAfterEdit(editedId)) {
      return;
    }
    focusMainInputAfterCommit();
  };

  const cancelEdit = (): void => {
    const returnSelectionId = editReturnSelectionIdRef.current;
    editReturnSelectionIdRef.current = undefined;
    setEditingId(undefined);
    setEditingText('');
    if (returnSelectionId !== undefined && restoreSelectionAfterEdit(returnSelectionId)) {
      return;
    }
    focusMainInputAfterCommit();
  };

  const enterSelectionMode = (): void => {
    if (todos.length === 0) {
      return;
    }

    mainInputRef.current?.blur();
    setEditingId(undefined);
    setSelectedIndex(0);
  };

  const moveSelectionDown = (): void => {
    setSelectedIndex(index => (
      index === undefined
        ? (todos.length > 0 ? 0 : undefined)
        : Math.min(todos.length - 1, index + 1)
    ));
  };

  const moveSelectionUp = (): void => {
    setSelectedIndex(index => {
      if (index === undefined) {
        return undefined;
      }
      if (index <= 0) {
        focusMainInputAfterCommit();
        return undefined;
      }
      return index - 1;
    });
  };

  const toggleTodo = (id: number): void => {
    setTodos(current => current.map(item => (
      item.id === id ? {...item, completed: !item.completed} : item
    )));
  };

  const deleteTodo = (id: number): void => {
    const nextSelectedIndex = indexAfterDelete(todos, id, selectedTodoId);
    setSelectedIndex(nextSelectedIndex);
    setTodos(current => current.filter(item => item.id !== id));
    if (nextSelectedIndex === undefined && selectedTodoId === id) {
      focusMainInputAfterCommit();
    }
    if (editingId === id) {
      cancelEdit();
    }
  };

  const selectedTodo = selectedIndex === undefined ? undefined : todos[selectedIndex];
  const selectionInstruction = selectionHint(selectedTodo);
  const selectedTodoId = selectedTodo?.id;

  return (
    <Div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 1,
        backgroundColor: '#0b1120',
        color: '#e5e7eb',
      }}
      onKeyDown={event => {
        if (editingId === undefined && selectedIndex !== undefined) {
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            moveSelectionDown();
            return;
          }
          if (event.key === 'ArrowUp') {
            event.preventDefault();
            moveSelectionUp();
            return;
          }
          if (selectedTodo !== undefined && event.key.toLowerCase() === 't') {
            event.preventDefault();
            toggleTodo(selectedTodo.id);
            return;
          }
          if (selectedTodo !== undefined && event.key.toLowerCase() === 'e') {
            event.preventDefault();
            if (!selectedTodo.completed) {
              beginEdit(selectedTodo, true);
            }
            return;
          }
          if (selectedTodo !== undefined && event.key.toLowerCase() === 'x') {
            event.preventDefault();
            deleteTodo(selectedTodo.id);
            return;
          }
        }
        if (event.key === 'Escape' && editingId !== undefined) {
          event.preventDefault();
          cancelEdit();
          return;
        }
        if (event.key === 'Escape' || (event.ctrlKey && event.code === 'KeyC')) {
          event.preventDefault();
          exit();
        }
      }}
    >
      <Div style={{width: 74, maxHeight: '90%', display: 'flex', flexDirection: 'column', gap: 1}}>
        <Span style={{color: '#38bdf8'}}>paintcannon-react todos</Span>
        <Form
          style={{
            display: 'flex',
            flexDirection: 'row',
            gap: 1,
            width: '100%',
          }}
          onSubmit={event => {
            event.preventDefault();
            addTodo();
          }}
        >
          <Input
            ref={mainInputRef}
            autoFocus
            value={draft}
            placeholder="Add a todo"
            style={{
              width: 58,
              height: 3,
              border: 'rounded',
              borderColor: '#475569',
              backgroundColor: mainInputFocused ? selectedRowBackground : '#020617',
              color: mainInputFocused ? '#ffffff' : '#f8fafc',
              placeholderColor: mainInputFocused ? '#94a3b8' : '#64748b',
            }}
            onChange={(event: PaintChangeEvent) => {
              setDraft(event.target.value);
            }}
            onFocus={() => {
              setMainInputFocused(true);
              setSelectedIndex(undefined);
            }}
            onBlur={() => {
              setMainInputFocused(false);
            }}
            onKeyDown={(event: PaintKeyboardEvent) => {
              if (event.key === 'ArrowDown') {
                event.preventDefault();
                event.stopPropagation();
                enterSelectionMode();
              }
            }}
          />
          <Button
            type="submit"
            style={addButtonStyle(hovered === 'add')}
            onMouseEnter={() => setHovered('add')}
            onMouseLeave={() => setHovered(undefined)}
          >
            Add
          </Button>
        </Form>
        <Div
          style={{
            display: 'flex',
            flexDirection: 'row',
            gap: 1,
            width: '100%',
            minHeight: 0,
            flexShrink: 1,
          }}
        >
          <Div
            ref={listRef}
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 1,
              width: 72,
              minHeight: 0,
              flexShrink: 1,
              overflowY: 'scroll',
              padding: '1 1',
              border: 'rounded',
              borderColor: '#334155',
              backgroundColor: '#111827',
            }}
            onScroll={(event: PaintScrollEvent) => {
              const list = listRef.current;
              updateScrollMetrics({
                scrollTop: event.scrollTop,
                scrollHeight: event.scrollHeight,
                clientHeight: list?.clientHeight ?? 0,
              });
            }}
          >
            {todos.length === 0 ? (
              <Div style={{color: '#64748b'}}>No todos</Div>
            ) : (
              todos.map(todo => (
                <TodoRow
                  key={todo.id}
                  selected={selectedTodoId === todo.id}
                  todo={todo}
                  editing={todo.id === editingId}
                  editingText={editingText}
                  setEditingText={setEditingText}
                  hovered={hovered}
                  setHovered={setHovered}
                  beginEdit={beginEdit}
                  commitEdit={commitEdit}
                  toggleTodo={() => {
                    toggleTodo(todo.id);
                  }}
                  deleteTodo={() => {
                    deleteTodo(todo.id);
                  }}
                />
              ))
            )}
          </Div>
          <ScrollRail metrics={scrollMetrics} />
        </Div>
        <Div style={{color: selectedIndex === undefined ? '#64748b' : '#38bdf8'}}>
          {selectionInstruction}
        </Div>
        <Div style={{color: '#64748b'}}>
          {todos.filter(todo => !todo.completed).length} open / {todos.length} total
        </Div>
      </Div>
    </Div>
  );
}

function ScrollRail({metrics}: {metrics: TodoScrollMetrics}): React.ReactElement {
  const clientHeight = Math.max(0, Math.floor(metrics.clientHeight));
  const scrollHeight = Math.max(clientHeight, Math.floor(metrics.scrollHeight));
  const overflow = scrollHeight > clientHeight && clientHeight > 0;
  const trackHeight = Math.max(1, clientHeight + todoListVerticalChrome);
  const thumbHeight = overflow
    ? Math.max(1, Math.floor((trackHeight * clientHeight) / scrollHeight))
    : trackHeight;
  const maxScrollTop = Math.max(0, scrollHeight - clientHeight);
  const maxThumbTop = Math.max(0, trackHeight - thumbHeight);
  const thumbTop = overflow && maxScrollTop > 0
    ? Math.round((metrics.scrollTop / maxScrollTop) * maxThumbTop)
    : 0;
  const thumbBottom = Math.max(0, trackHeight - thumbTop - thumbHeight);

  return (
    <Div
      style={{
        width: 1,
        height: trackHeight,
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        backgroundColor: '#111827',
      }}
    >
      <Div style={{height: thumbTop, width: 1, flexShrink: 0}} />
      <Div
        style={{
          height: thumbHeight,
          width: 1,
          flexShrink: 0,
          backgroundColor: overflow ? '#38bdf8' : '#334155',
        }}
      />
      <Div style={{height: thumbBottom, width: 1, flexShrink: 0}} />
    </Div>
  );
}

const todoListVerticalChrome = 2;
const todoRowHeight = 3;
const todoRowStride = 4;
const selectedRowBackground = '#1e3a5f';

function selectionHint(selectedTodo: Todo | undefined): string {
  if (selectedTodo === undefined) {
    return 'Press Down to select todos';
  }

  return selectedTodo.completed
    ? 'Press Up or Down to select items; T toggles, X deletes'
    : 'Press Up or Down to select items; T toggles, E edits, X deletes';
}

function indexAfterDelete(
  todos: Todo[],
  deletedId: number,
  selectedId: number | undefined,
): number | undefined {
  if (selectedId === undefined) {
    return undefined;
  }

  const deletedIndex = todos.findIndex(todo => todo.id === deletedId);
  if (deletedIndex === -1) {
    const selectedIndex = todos.findIndex(todo => todo.id === selectedId);
    return selectedIndex === -1 ? undefined : selectedIndex;
  }

  const nextTodos = todos.filter(todo => todo.id !== deletedId);
  if (nextTodos.length === 0) {
    return undefined;
  }

  if (selectedId === deletedId) {
    return Math.max(0, deletedIndex - 1);
  }

  const selectedNextIndex = nextTodos.findIndex(todo => todo.id === selectedId);
  return selectedNextIndex === -1 ? undefined : selectedNextIndex;
}

function TodoRow({
  todo,
  selected,
  editing,
  editingText,
  setEditingText,
  hovered,
  setHovered,
  beginEdit,
  commitEdit,
  toggleTodo,
  deleteTodo,
}: {
  todo: Todo;
  selected: boolean;
  editing: boolean;
  editingText: string;
  setEditingText: React.Dispatch<React.SetStateAction<string>>;
  hovered: HoverTarget | undefined;
  setHovered: React.Dispatch<React.SetStateAction<HoverTarget | undefined>>;
  beginEdit(todo: Todo): void;
  commitEdit(): void;
  toggleTodo(): void;
  deleteTodo(): void;
}): React.ReactElement {
  const checkHover = hovered === `check:${todo.id}`;
  const editHover = hovered === `edit:${todo.id}`;
  const deleteHover = hovered === `delete:${todo.id}`;

  return (
    <Div
      style={{
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        gap: 1,
        width: '100%',
        flexShrink: 0,
        backgroundColor: selected ? selectedRowBackground : undefined,
      }}
    >
      <Button
        type="button"
        style={checkboxStyle(todo.completed, checkHover)}
        onMouseEnter={() => setHovered(`check:${todo.id}`)}
        onMouseLeave={() => setHovered(undefined)}
        onClick={event => {
          event.preventDefault();
          toggleTodo();
        }}
      >
        {todo.completed ? '✓' : ' '}
      </Button>
      {editing ? (
        <Input
          autoFocus
          value={editingText}
          style={{
            flexGrow: 1,
            flexShrink: 1,
            height: 3,
            border: 'rounded',
            borderColor: '#38bdf8',
            backgroundColor: '#020617',
            color: '#f8fafc',
          }}
          onChange={(event: PaintChangeEvent) => {
            setEditingText(event.target.value);
          }}
          onKeyDown={(event: PaintKeyboardEvent) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              commitEdit();
            }
          }}
          onBlur={() => {
            commitEdit();
          }}
        />
      ) : (
        <Div
          style={{
            flexGrow: 1,
            flexShrink: 1,
            color: todo.completed ? '#64748b' : '#f8fafc',
          }}
        >
          {todo.completed ? `${todo.text} (done)` : todo.text}
        </Div>
      )}
      {todo.completed ? (
        <Div style={{width: 3, height: 3, flexShrink: 0}} />
      ) : (
        <Button
          type="button"
          style={iconButtonStyle('edit', editHover)}
          onMouseEnter={() => setHovered(`edit:${todo.id}`)}
          onMouseLeave={() => setHovered(undefined)}
          onClick={event => {
            event.preventDefault();
            beginEdit(todo);
          }}
        >
          ✏
        </Button>
      )}
      <Button
        type="button"
        style={iconButtonStyle('delete', deleteHover)}
        onMouseEnter={() => setHovered(`delete:${todo.id}`)}
        onMouseLeave={() => setHovered(undefined)}
        onClick={event => {
          event.preventDefault();
          deleteTodo();
        }}
      >
        x
      </Button>
    </Div>
  );
}

function addButtonStyle(hovered: boolean): CSSStyleProperties {
  const backgroundColor = hovered ? '#15803d' : '#14532d';
  return {
    width: 14,
    height: 3,
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    border: 'chunky-rounded',
    borderColor: backgroundColor,
    backgroundColor,
    color: '#f0fdf4',
    cursor: 'pointer',
  };
}

function checkboxStyle(completed: boolean, hovered: boolean): CSSStyleProperties {
  if (completed) {
    const backgroundColor = hovered ? '#166534' : '#14532d';
    return {
      width: 3,
      height: 3,
      flexShrink: 0,
      border: 'chunky-rounded',
      borderColor: backgroundColor,
      backgroundColor,
      color: '#dcfce7',
      cursor: 'pointer',
    };
  }

  const backgroundColor = hovered ? '#1e293b' : '#020617';
  return {
    width: 3,
    height: 3,
    flexShrink: 0,
    border: 'chunky-rounded',
    borderColor: backgroundColor,
    backgroundColor,
    color: '#94a3b8',
    cursor: 'pointer',
  };
}

function iconButtonStyle(kind: 'edit' | 'delete', hovered: boolean): CSSStyleProperties {
  if (kind === 'edit') {
    const backgroundColor = hovered ? '#713f12' : '#1e293b';
    return {
      width: 3,
      height: 3,
      flexShrink: 0,
      border: 'chunky-rounded',
      borderColor: backgroundColor,
      backgroundColor,
      color: '#fef3c7',
      cursor: 'pointer',
    };
  }

  const backgroundColor = hovered ? '#7f1d1d' : '#450a0a';
  return {
    width: 3,
    height: 3,
    flexShrink: 0,
    border: 'chunky-rounded',
    borderColor: backgroundColor,
    backgroundColor,
    color: '#fee2e2',
    cursor: 'pointer',
  };
}

const root = render(<TodoApp />, {
  alternateScreen: true,
  captureCtrlC: true,
  captureMouse: true,
});

root.waitUntilExit().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
