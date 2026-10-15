-- Compila (sem executar) todos os .luau de uma pasta e conta os erros de sintaxe.
local fs = require("@lune/fs")
local luau = require("@lune/luau")
local process = require("@lune/process")

local root = process.args[1]
local total, failed = 0, 0

local function walk(dir: string)
	for _, name in fs.readDir(dir) do
		local path = dir .. "/" .. name
		if fs.isDir(path) then
			walk(path)
		elseif string.match(name, "%.luau?$") then
			total += 1
			local ok, err = pcall(luau.compile, fs.readFile(path))
			if not ok then
				failed += 1
				print("ERRO", path, err)
			end
		end
	end
end

walk(root)
print(`arquivos={total} com_erro={failed}`)
