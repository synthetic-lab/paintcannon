import React, {useCallback, useEffect, useRef, useState} from 'react';
import {
  Button,
  Div,
  Form,
  Input,
  Span,
  render,
  useApp,
  type CSSStyleProperties,
  type DivElement,
  type PaintChangeEvent,
  type PaintKeyboardEvent,
  type PaintScrollEvent,
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
  const [draft, setDraft] = useState('');
  const [todos, setTodos] = useState<Todo[]>([
    {id: 1, text: 'Ship the React reconciler', completed: true},
    {id: 2, text: 'Make controlled inputs feel native', completed: false},
    {id: 3, text: 'Replace terminal UI hacks with paintcannon', completed: false},
  ]);
  const [nextId, setNextId] = useState(4);
  const [editingId, setEditingId] = useState<number | undefined>();
  const [editingText, setEditingText] = useState('');
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
    const handleResize = (): void => readListScrollMetrics();
    paintCannon.addEventListener('resize', handleResize);
    return () => paintCannon.removeEventListener('resize', handleResize);
  }, [paintCannon, readListScrollMetrics]);

  const addTodo = (): void => {
    const text = draft.trim();
    if (text.length === 0) {
      return;
    }

    setTodos(current => [...current, {id: nextId, text, completed: false}]);
    setNextId(id => id + 1);
    setDraft('');
  };

  const beginEdit = (todo: Todo): void => {
    setEditingId(todo.id);
    setEditingText(todo.text);
  };

  const commitEdit = (): void => {
    if (editingId === undefined) {
      return;
    }

    const text = editingText.trim();
    if (text.length === 0) {
      setTodos(current => current.filter(todo => todo.id !== editingId));
    } else {
      setTodos(current => current.map(todo => (
        todo.id === editingId ? {...todo, text} : todo
      )));
    }
    setEditingId(undefined);
    setEditingText('');
  };

  const cancelEdit = (): void => {
    setEditingId(undefined);
    setEditingText('');
  };

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
            autoFocus
            value={draft}
            placeholder="Add a todo"
            style={{
              width: 58,
              height: 3,
              border: 'rounded',
              borderColor: '#475569',
              backgroundColor: '#020617',
              color: '#f8fafc',
              placeholderColor: '#64748b',
            }}
            onChange={(event: PaintChangeEvent) => {
              setDraft(event.target.value);
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
                  todo={todo}
                  editing={todo.id === editingId}
                  editingText={editingText}
                  setEditingText={setEditingText}
                  hovered={hovered}
                  setHovered={setHovered}
                  beginEdit={beginEdit}
                  commitEdit={commitEdit}
                  toggleTodo={() => {
                    setTodos(current => current.map(item => (
                      item.id === todo.id ? {...item, completed: !item.completed} : item
                    )));
                  }}
                  deleteTodo={() => {
                    setTodos(current => current.filter(item => item.id !== todo.id));
                    if (editingId === todo.id) {
                      cancelEdit();
                    }
                  }}
                />
              ))
            )}
          </Div>
          <ScrollRail metrics={scrollMetrics} />
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

function TodoRow({
  todo,
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
            width: 54,
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
            width: 54,
            color: todo.completed ? '#64748b' : '#f8fafc',
          }}
        >
          {todo.completed ? `${todo.text} (done)` : todo.text}
        </Div>
      )}
      {todo.completed ? (
        <Div style={{width: 3, height: 3}} />
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
