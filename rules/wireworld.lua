Rule = function(c, n)
  if c == 0 then
    return 0
  elseif c == 1 then
    local powered = 0
    for _, x in ipairs(n) do
      if x == 2 then
        powered = powered + 1
      end
    end
    if powered == 1 or powered == 2 then
      return 2
    else
      return 1
    end
  elseif c == 2 then
    return 3
  else
    return 1
  end
end

Display = function(n)
  if n == 0 then return "  " end
  if n == 1 then return "[]" end
  if n == 2 then return "**" end
  if n == 3 then return ".." end
end

States = 4
