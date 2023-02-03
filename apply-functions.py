functions = {}
functions['18446744073709551615'] = ''
functions['0xffffffffffffffff'] = ''
with open('functions.txt', 'r') as funcfile:
    for line in funcfile.readlines():
        functions[line[:4]] = line[4:-1]
print(functions)
codelines = []
with open('compiled.wat', 'r') as wat:
    for line in wat.readlines():
        for func in functions.keys():
            line = line.replace(func, functions[func])
        codelines.append(line)

with open('applyed.wat', 'w') as res:
    res.writelines(codelines)
