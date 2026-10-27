import Quill from "quill";
import Block from "quill/blots/block";
import { SCENE_BREAK, type SceneId } from "./script_layout";

export const sceneTitles = new Map<SceneId, string>();

class SceneBreak extends Block {
  static override blotName = SCENE_BREAK;
  static override tagName = "DIV";
  static override className = "ql-scene-break";

  static override create(value: unknown) {
    const node = super.create(value) as HTMLElement;
    node.setAttribute("contenteditable", "false");
    node.dataset.sceneId = String(value);
    node.dataset.title = sceneTitles.get(Number(value)) ?? "";
    return node;
  }

  static override formats(node: HTMLElement) {
    return Number(node.dataset.sceneId);
  }
}

Quill.register(SceneBreak, true);
