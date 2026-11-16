-- bld.factory@modern: Modern factory (mesh.factory_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.factory_modern','Modern factory');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.factory_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.tarmac',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.precast',0,'','{"x":-0.15,"y":0.05,"z":-0.45,"w":3,"d":2.4,"h":1.35,"windows":1,"window_w":2.6,"window_h":0.26,"window_y":0.82,"side_windows":0,"opening":"#2b3b49"}'),
  ('doors','box','mat.white_metal',0,'','{"x":-1,"y":0.05,"z":0.765,"w":0.55,"d":0.02,"h":0.62,"repeat":3,"dx":0.75}'),
  ('roof_slopes','box','mat.cladding',0,'','{"x":-0.15,"y":1.4,"z":-1.65,"w":2.92,"d":0.05,"h":0.75,"pitch":53.13,"repeat":4,"dz":0.6}'),
  ('roof_glass','box','mat.curtain_wall',0,'','{"x":-0.15,"y":1.4,"z":-1.07,"w":2.92,"d":0.04,"h":0.45,"repeat":4,"dz":0.6}'),
  ('parapets','box','mat.cloth',1,'','{"x":-1.6,"y":1.4,"z":-0.45,"w":0.12,"d":2.4,"h":0.5,"repeat":2,"dx":2.9}'),
  ('parapet_caps','box','mat.cloth',1,'','{"x":-1.6,"y":1.9,"z":-0.45,"w":0.16,"d":2.44,"h":0.06,"repeat":2,"dx":2.9,"far":0}'),
  ('chimney','cylinder','mat.concrete',0,'','{"x":1.5,"y":0.05,"z":-1.35,"w":0.5,"h":4.8,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":1.5,"y":3.698,"z":-1.35,"w":0.408,"h":0.48,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":1.5,"y":4.85,"z":-1.35,"w":0.364,"h":0.1,"segments":8,"far":0}'),
  ('chimney_light','box','mat.signal_red',0,'','{"x":1.5,"y":4.95,"z":-1.35,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('chimney_2','cylinder','mat.steel',0,'','{"x":1.5,"y":0.05,"z":-0.45,"w":0.32,"h":3.4,"taper":0.1,"segments":8}'),
  ('chimney_2_band','cylinder','mat.cloth',1,'','{"x":1.5,"y":2.634,"z":-0.45,"w":0.317,"h":0.34,"segments":8}'),
  ('chimney_2_cap','cylinder','mat.cast_iron',0,'','{"x":1.5,"y":3.45,"z":-0.45,"w":0.3,"h":0.1,"segments":8,"far":0}'),
  ('office','box','mat.curtain_wall',0,'','{"x":-1.05,"y":0.05,"z":1.25,"w":1.3,"d":0.65,"h":0.95}'),
  ('office_fascia','box','mat.cloth',1,'','{"x":-1.05,"y":1,"z":1.25,"w":1.36,"d":0.71,"h":0.14}'),
  ('office_roof','flat_roof','mat.cloth',1,'','{"x":-1.05,"y":1.14,"z":1.25,"w":1.36,"d":0.71,"t":0.03,"overhang":0}'),
  ('trailer','box','mat.cloth',1,'','{"x":0.65,"y":0.16,"z":1.25,"w":1.2,"d":0.38,"h":0.42}'),
  ('cab','box','mat.white_metal',0,'','{"x":1.45,"y":0.12,"z":1.25,"w":0.32,"d":0.38,"h":0.4,"tint":"#3a5a8a"}'),
  ('cab_glass','box','mat.curtain_wall',0,'','{"x":1.6,"y":0.32,"z":1.25,"w":0.04,"d":0.34,"h":0.15,"far":0}'),
  ('wheels','cylinder','mat.rubber',0,'','{"x":0.25,"y":0.11,"z":1.05,"w":0.22,"h":0.4,"segments":6,"pitch":90,"repeat":3,"dx":0.55}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.factory_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.factory'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.factory_modern'), (SELECT id FROM def WHERE key='mat.precast'), 100);
