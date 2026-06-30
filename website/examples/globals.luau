-- globals.luau
-- Iterate _G, the table holding every global. A nod to the reader who reported
-- that this snippet's output used to show up red: the globals include one
-- literally named "error", and an old build classified a run as failed by
-- scanning the output text for words like that. Now success vs. error is decided
-- from a typed result, never the output's contents — so this prints just fine.

print(_G)
for i, v in _G do
	print(i)
	print(v)
end
print("Done")
