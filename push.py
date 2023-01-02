import os
import argparse


parser = argparse.ArgumentParser()
parser.add_argument('-m', '--commit')
commit_text = parser.parse_args().commit
os.system("cargo fmt")
os.system("git add *")
os.system(f"git commit -m {commit_text}")
os.system("git push")
