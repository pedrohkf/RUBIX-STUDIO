local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Workspace = game:GetService("Workspace")

local Config = require(ReplicatedStorage.Shared.Config)
local HealthColor = require(ReplicatedStorage.Shared.HealthColor)

local function part(name: string, size: Vector3, position: Vector3, color: Color3): Part
	local p = Instance.new("Part")
	p.Name = name
	p.Size = size
	p.Position = position
	p.Color = color
	p.Anchored = true
	p.Parent = Workspace
	return p
end

-- Chão: o topo fica em y = 0.
part("Ground", Config.Ground.Size, Vector3.new(0, -Config.Ground.Size.Y / 2, 0), Config.Ground.Color)

-- Pilares coloridos pela faixa de vida (usa o módulo compartilhado).
for i, fraction in { 1, 0.5, 0.2, 0.05 } do
	part(`Pilar{i}`, Vector3.new(3, 8, 3), Vector3.new(-18 + i * 9, 4, 12), HealthColor.of(fraction))
end

-- Bola
local ball = part("Bola", Vector3.new(5, 5, 5), Vector3.new(22, 2.5, -8), Color3.fromRGB(70, 130, 230))
ball.Shape = Enum.PartType.Ball

-- Cubo girando, movido pelo script a cada quadro lógico (editar Config.Cube.Color atualiza ao vivo).
local cube = part(Config.Cube.Name or "Cubo", Config.Cube.Size, Config.Cube.Position, Config.Cube.Color)
task.spawn(function()
	local angle = 0
	while true do
		angle += Config.Cube.SpinSpeed / 30
		local lift = 1 + math.sin(math.rad(angle * 2)) * 1
		cube.CFrame = CFrame.new(Config.Cube.Position + Vector3.new(0, lift, 0)) * CFrame.Angles(0, math.rad(angle), 0)
		task.wait(1 / 30)
	end
end)

print(`[{Config.GameName}] servidor iniciado v{Config.Version}`)
