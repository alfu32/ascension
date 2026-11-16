-- bld.library@modern: Modern library (mesh.library_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_modern','Modern library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.concrete',0,'','{"w":3.6,"d":3.4,"h":0.25,"steps":2,"inset":0.12}'),
  ('block','box','mat.precast',0,'','{"y":0.25,"z":-0.15,"w":2.8,"d":2.4,"h":1.7,"windows":3,"window_w":0.12,"window_h":1.05,"window_y":0.35,"side_windows":3,"opening":"#2b3b49"}'),
  ('hangings','box','mat.cloth',1,'','{"x":-1.05,"y":0.45,"z":1.07,"w":0.46,"d":0.03,"h":1.25,"repeat":2,"dx":2.1}'),
  ('frieze','box','mat.cloth',1,'','{"y":1.85,"z":-0.15,"w":2.9,"d":2.5,"h":0.3}'),
  ('roof','flat_roof','mat.concrete',0,'','{"y":2.15,"z":-0.15,"w":2.9,"d":2.5,"t":0.06,"overhang":0}'),
  ('parapet','wall','mat.cloth',1,'','{"y":2.21,"z":-0.15,"w":2.8,"d":2.4,"h":0.12,"t":0.1,"ring":1}'),
  ('drum','cylinder','mat.cloth',1,'','{"y":2.21,"z":-0.15,"w":1.9,"h":0.32,"segments":16}'),
  ('drum_glass','cylinder','mat.curtain_wall',0,'','{"y":2.29,"z":-0.15,"w":1.92,"h":0.14,"segments":16,"far":0}'),
  ('dome','dome','mat.concrete',0,'','{"y":2.53,"z":-0.15,"w":2,"h":0.75,"segments":16,"tint":"#ece8e0"}'),
  ('lantern','cylinder','mat.curtain_wall',0,'','{"y":3.18,"z":-0.15,"w":0.44,"h":0.24,"segments":8}'),
  ('lantern_cap','cylinder','mat.cloth',1,'','{"y":3.42,"z":-0.15,"w":0.56,"h":0.07,"segments":8}'),
  ('lobby','box','mat.curtain_wall',0,'','{"y":0.25,"z":1.33,"w":1.6,"d":0.56,"h":0.95}'),
  ('lobby_roof','flat_roof','mat.cloth',1,'','{"y":1.2,"z":1.3,"w":1.8,"d":0.8,"t":0.09,"overhang":0}'),
  ('lobby_posts','cylinder','mat.steel',0,'','{"x":-0.8,"y":0.25,"z":1.6,"w":0.06,"h":0.95,"segments":5,"repeat":2,"dx":1.6,"far":0}'),
  ('doors','box','mat.cast_iron',0,'','{"y":0.25,"z":1.62,"w":0.5,"d":0.02,"h":0.6,"far":0}'),
  ('lamps','cylinder','mat.steel',0,'','{"x":-1.55,"y":0.25,"z":1.55,"w":0.04,"h":0.9,"segments":4,"repeat":2,"dx":3.1,"far":0}'),
  ('lamp_heads','box','mat.white_metal',0,'','{"x":-1.55,"y":1.15,"z":1.55,"w":0.14,"d":0.14,"h":0.06,"repeat":2,"dx":3.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.library_modern'), (SELECT id FROM def WHERE key='mat.precast'), 100);
