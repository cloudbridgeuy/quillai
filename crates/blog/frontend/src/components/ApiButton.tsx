import { useCallback, useReducer } from "react";
import type {PropsWithChildren} from "react";

export type ControllerPropsT = PropsWithChildren<{
  className?: string;
}>

type Post = {
  title: string,
  content: string,
}

enum ActionType {
  NoOp = "NO_OP",
  FetchPosts = "FETCH_POSTS",
  FetchSuccess = "FETCH_SUCCESS",
  FetchError = "FETCH_ERROR",
}

type Action = {
  type: ActionType;
  payload?: Partial<State>;
}

enum Event {
  Idle = "IDLE",
  Loading = "LOADING",
  Error = "ERROR",
}

type State = {
  posts: Post[];
  event: Event;
}

function reducer(state: State, action: Action = {type: ActionType.NoOp}) {
  const {type, payload = {}} = action;

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

function Container(props: ControllerPropsT) {
  const [state, dispatch] = useReducer(reducer, {posts: [], event: Event.Idle});
  const handleClick = useCallback(async () => {
    const response = await fetch('/api/posts');

    if (response.status === 200) {
      const data = await response.json()
      dispatch({type: ActionType.FetchSuccess, payload: {posts: data}});
    } else {
      dispatch({type: ActionType.FetchError});
    }
  }, []);

  return <Component loading={state.event === Event.Loading} onClick={handleClick} posts={state.posts} {...props}/>
}

export type ComponentPropsT = ControllerPropsT & {
  onClick: () => Promise<void>;
  posts: Post[];
  loading?: boolean;
};

function Component({posts = [], loading = false, children, className, onClick}: ComponentPropsT) {
  return (
    <div>
      <button onClick={onClick} type="button" className={className}>{loading ? "Loading..." : children}</button>
      {posts.length > 0 && <pre>{JSON.stringify(posts, null, 2)}</pre>}
    </div>
  );
}

export default Container;
