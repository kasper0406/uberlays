import bpy
import sys

sys.path.append('./scripts/build/')
import track_pb2

blend_curves = [ obj for obj in bpy.context.scene.objects if obj.type == "CURVE" ]

def min_max(prev, point):
    return (
        min(min(prev[0], point[0]), point[1]),
        max(max(prev[1], point[0]), point[1])
    )

def scale(scale, point):
    len = scale[1] - scale[0]
    return ( (point[0] - scale[0]) / len, (point[1] - scale[0]) / len )

def set_point(proto_point, point):
    proto_point.x = point[0]
    proto_point.y = point[1]

track = track_pb2.Track()
assert(len(blend_curves) == 1)
for i, obj in enumerate(blend_curves):
    assert(len(obj.data.splines) == 1)
    spline = obj.data.splines[0]

    min_val = 100000
    max_val = -100000
    
    for point in spline.bezier_points:
        min_val, max_val = min_max((min_val, max_val), point.handle_left)
        min_val, max_val = min_max((min_val, max_val), point.co)
        min_val, max_val = min_max((min_val, max_val), point.handle_right)

    r = (min_val, max_val)
    print('r:', r)

    for j, point in enumerate(spline.bezier_points):
        bezier_point = track.curve.add()
        set_point(bezier_point.handle_left, scale(r, point.handle_left))
        set_point(bezier_point.control, scale(r, point.co))
        set_point(bezier_point.handle_right, scale(r, point.handle_right))

with open('output.dat', 'w+b') as file:
    file.write(track.SerializeToString())
