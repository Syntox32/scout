import ast
from asyncio.proactor_events import constants
import base64
from stat import FILE_ATTRIBUTE_SPARSE_FILE
import sys
from typing import *
import re
import os.path

def get_canaries() -> List[str]:
  with open("canary/canary.lst", "r", encoding="utf-8") as f:
    return [c.replace("\n", "").replace("\r", "") for c in f.readlines() if c != ""]

def lookup_canary(original_cnary: str, match: str) -> List[str]:
  with open("canary/lookup.lst", "r", encoding="utf-8") as f:
    return [line for line in f.readlines() if original_cnary in line]

def collect_constants(ast_module: ast.AST) -> List[str]:
  ignore = [line.replace("\n", "").replace("\r", "") \
          for line in open("canary/canary.ignore", "r").readlines()]
  constants: List[str] = []
  for node in ast.walk(ast_module):
    if isinstance(node, ast.Constant):
      if node.value not in ignore \
          and str(node.value) != "" \
          and len(str(node.value)) >= 3 \
          and not str(node.value).isdigit():
        constants.append(str(node.value))
  return constants

pattern = re.compile(r"^([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{3}=|[A-Za-z0-9+/]{2}==)?$", re.MULTILINE)

def probably_base64(value):
  return re.match(pattern, value)

def is_code(value):
  try:
    return ast.parse(value)
  except:
    #print(f"Code, yes, but got error: {value}")
    return None
  return None

def check_canary(canaries, constant, f):
  found_something = False

  if probably_base64(constant):
    #print("^this is probably base64")
    if is_code(base64.b64decode(constant)):
      print(f"interesting code found: {base64.b64decode(constant)}")
      found_something = True
  #print(constant)
  #print(canaries)

  header = False
  for canary in canaries:
    if canary in constant:
      if not header:
        print("="*40)
        print(f"checking file {f}")
        header = True
      text = constant
      if len(text) > 15:
        text = constant[:15] + "..."
      print(f"Lookup '{text}': {lookup_canary(canary, constant)}")
      found_something = True

  return found_something

def invalidate_file(file):
  with open(file, "r", encoding="utf-8") as f:
    src = f.read()
    module = ast.parse(src)
    #print(ast.dump(module, indent=4))

    #for node in ast.walk(module):
      #if isinstance(node, ast.Call):
        #if "id" in node.func._fields:
        # print(node.func.id)
        #for cnode in ast.iter_child_nodes(node):
          #if "id" in cnode._fields:
            #print("\t", cnode.id)
          #else:
          #  print(cnode)

  canaries = get_canaries() #open("canary/canary.lst", "r").read()
  constants = collect_constants(module)
  header = False
  found_something = False

  #print(canaries)

  #for canary in canaries:
  #  if canary in "dark-magic":

  for constant in constants:
    #print(constant)
  #  if :
    found_something = check_canary(canaries, constant, file)
  #  break
    #if re.search(constant, canaries):
  
  return found_something

if __name__ == "__main__":
  invalidate_file(os.path.abspath(sys.argv[1]))