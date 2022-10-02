local survive = { [2] = true, [3] = true };
local born = { [3] = true };

Rule = function(c, n)
  local sum = 0
  for _, v in ipairs(n) do
    sum = sum + v
  end
  if c == 0 then
    if born[sum] then
      return 1
    else
      return 0
    end
  else
    if survive[sum] then
      return 1
    else
      return 0
    end
  end
end

Display = function(n)
  if n == 0 then return "  " end
  if n == 1 then return "()" end
end

States = 2
