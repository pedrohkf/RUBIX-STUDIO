local ReplicatedStorage = game:GetService("ReplicatedStorage")

local Config = require(ReplicatedStorage.Shared.Config)

print(`[{Config.GameName}] cliente iniciado v{Config.Version}`)
