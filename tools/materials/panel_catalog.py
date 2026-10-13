"""Resolve a faction panel catalogue over the shared placement and bake rules.

Both the Blender builder and HTML inliner read this result, so the asset paths,
layer numbering and inherited dimensions cannot diverge between them.
"""
import copy
import json
from pathlib import Path


def load_catalog(path):
    path=Path(path).resolve()
    data=json.loads(path.read_text(encoding='utf-8'))
    parent=data.pop('extends',None)
    if not parent:return data
    base=load_catalog(path.parent/parent)
    def merge(target,overrides):
        for key,value in overrides.items():
            if isinstance(value,dict) and isinstance(target.get(key),dict):merge(target[key],value)
            else:target[key]=copy.deepcopy(value)
    merge(base,data)
    return base
