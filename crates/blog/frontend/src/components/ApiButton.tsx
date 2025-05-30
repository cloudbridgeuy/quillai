import { useCallback, useReducer } from "react";
import type { PropsWithChildren } from "react";

// Define the `Post` type
export type Post = {
  title: string;
  content: string;
};

// Enums for ActionType and Event
enum ActionType {
  NoOp = "NO_OP",
  FetchPosts = "FETCH_POSTS",
  FetchSuccess = "FETCH_SUCCESS",
  FetchError = "FETCH_ERROR",
}

enum Event {
  Idle = "IDLE",
  Loading = "LOADING",
  Error = "ERROR",
}

// Define the `State` shape
type State = {
  posts: Post[];
  event: Event;
  error?: string;
};

// Define the `Action` shape
type Action = {
  type: ActionType;
  payload?: Partial<State>;
};

// Reducer function to handle state transitions
function reducer(state: State, action: Action = { type: ActionType.NoOp }): State {
  const { type, payload = {} } = action;

  switch (type) {
    case ActionType.FetchPosts:
      return {
        ...state,
        event: Event.Loading,
      };
    case ActionType.FetchSuccess:
      return {
        ...state,
        posts: payload.posts || [],
        event: Event.Idle,
      };
    case ActionType.FetchError:
      return {
        ...state,
        event: Event.Error,
      };
    default:
      return state;
  }
}

// Props for the `Container` Component
export type ControllerPropsT = PropsWithChildren<{
  className?: string; // Optional CSS class for the component
}>;

// Parent container component which handles state & logic
function Container(props: ControllerPropsT) {
  const [state, dispatch] = useReducer(reducer, { posts: [], event: Event.Idle });

  // Handle click to fetch posts
  const handleClick = useCallback(async () => {
    dispatch({ type: ActionType.FetchPosts });

    try {
      const response = await fetch("/api/posts");
      if (response.ok) {
        const data = await response.json();
        dispatch({ type: ActionType.FetchSuccess, payload: { posts: data } });
      } else {
        dispatch({ type: ActionType.FetchError, payload: { error: "Failed to fetch posts" } });
      }
    } catch {
      dispatch({ type: ActionType.FetchError, payload: { error: "Network error" } });
    }
  }, []);

  return (
    <Component
      loading={state.event === Event.Loading}
      onClick={handleClick}
      posts={state.posts}
      {...props}
    />
  );
}

// Props for the presentation `Component`
export type ComponentPropsT = ControllerPropsT & {
  onClick: () => Promise<void>;
  posts: Post[];
  loading?: boolean; // Optional loading flag
};

// Presentation component for UI rendering
function Component({
  posts = [],
  loading = false,
  children,
  className,
  onClick,
}: ComponentPropsT) {
  return (
    <div>
      <button onClick={onClick} type="button" className={className}>
        {loading ? "Loading..." : children}
      </button>

      {posts.length > 0 && <pre>{JSON.stringify(posts, null, 2)}</pre>}
    </div>
  );
}

// Export the Container as the default export
export default Container;
