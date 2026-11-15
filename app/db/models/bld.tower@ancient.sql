-- bld.tower@ancient: Early watchtower (mesh.tower_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_ancient','Early watchtower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('footings','box','mat.fieldstone',0,'','{"x":-0.68,"z":-0.68,"w":0.3,"d":0.3,"h":0.14,"repeat":2,"dx":1.36,"far":0}'),
  ('footings_front','box','mat.fieldstone',0,'','{"x":-0.68,"z":0.68,"w":0.3,"d":0.3,"h":0.14,"repeat":2,"dx":1.36,"far":0}'),
  ('leg_nw','cylinder','mat.timber',0,'','{"x":-0.68,"z":-0.68,"w":0.17,"h":3.25,"segments":6,"roll":3,"pitch":3}'),
  ('leg_ne','cylinder','mat.timber',0,'','{"x":0.68,"z":-0.68,"w":0.17,"h":3.25,"segments":6,"roll":-3,"pitch":3}'),
  ('leg_sw','cylinder','mat.timber',0,'','{"x":-0.68,"z":0.68,"w":0.17,"h":3.25,"segments":6,"roll":3,"pitch":-3}'),
  ('leg_se','cylinder','mat.timber',0,'','{"x":0.68,"z":0.68,"w":0.17,"h":3.25,"segments":6,"roll":-3,"pitch":-3}'),
  ('brace_a','box','mat.timber',0,'','{"x":-0.662,"y":0.35,"z":0.6,"w":0.07,"d":0.07,"h":2.43,"roll":30,"repeat":2,"dz":-1.2}'),
  ('brace_b','box','mat.timber',0,'','{"x":0.662,"y":0.35,"z":0.6,"w":0.07,"d":0.07,"h":2.43,"roll":-30,"repeat":2,"dz":-1.2}'),
  ('brace_c','box','mat.timber',0,'','{"x":-0.6,"y":0.35,"z":-0.662,"w":0.07,"d":0.07,"h":2.43,"pitch":30,"repeat":2,"dx":1.2,"far":0}'),
  ('brace_d','box','mat.timber',0,'','{"x":-0.6,"y":0.35,"z":0.662,"w":0.07,"d":0.07,"h":2.43,"pitch":-30,"repeat":2,"dx":1.2,"far":0}'),
  ('platform','box','mat.timber',0,'','{"y":3.2,"w":1.5,"d":1.5,"h":0.12}'),
  ('parapet','palisade','mat.timber',0,'','{"y":3.32,"w":1.5,"d":1.5,"h":0.55,"t":0.08,"spacing":0.25,"ring":1}'),
  ('shield_front','box','mat.cloth',1,'','{"y":3.4,"z":0.78,"w":0.9,"d":0.04,"h":0.4}'),
  ('shield_sides','box','mat.cloth',1,'','{"x":-0.78,"y":3.4,"yaw":90,"w":0.9,"d":0.04,"h":0.4,"repeat":2,"dx":1.56}'),
  ('roof_posts','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.32,"z":-0.62,"w":0.08,"h":0.95,"segments":4,"repeat":2,"dx":1.24,"far":0}'),
  ('roof_posts_front','cylinder','mat.timber',0,'','{"x":-0.62,"y":3.32,"z":0.62,"w":0.08,"h":0.95,"segments":4,"repeat":2,"dx":1.24,"far":0}'),
  ('roof','hip_roof','mat.thatch',0,'','{"y":4.27,"w":1.3,"d":1.3,"pitch":42,"overhang":0.25,"t":0.1}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"y":4.612,"w":0.84,"d":0.84,"pitch":42,"overhang":0,"t":0.03}'),
  ('banner','banner','mat.cloth',1,'','{"y":4.75,"h":1.25,"fw":0.5,"fh":0.55}'),
  ('ladder','fence','mat.timber',0,'','{"x":0.2,"y":1.6,"z":0.82,"w":3.2,"h":0.4,"spacing":0.35,"rails":2,"roll":-90,"pitch":-3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.tower_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
