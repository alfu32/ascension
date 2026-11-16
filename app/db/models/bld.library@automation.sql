-- bld.library@automation: Future library (mesh.library_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.library_automation','Future library');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.library_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.paving',0,'','{"w":3.6,"d":3.4,"h":0.25,"steps":2,"inset":0.12}'),
  ('block','box','mat.composite_panel',0,'','{"y":0.25,"z":-0.15,"w":2.8,"d":2.4,"h":1.6,"windows":3,"window_w":0.12,"window_h":1,"window_y":0.3,"side_windows":3,"opening":"#2c5258"}'),
  ('block_glow','box','mat.light_strip',0,'','{"y":0.25,"z":-0.15,"w":2.82,"d":2.42,"h":0.03,"far":0}'),
  ('hangings','box','mat.white_composite',1,'','{"x":-1.05,"y":0.45,"z":1.07,"w":0.4,"d":0.03,"h":1.2,"repeat":2,"dx":2.1}'),
  ('frieze','box','mat.white_composite',1,'','{"y":1.85,"z":-0.15,"w":2.9,"d":2.5,"h":0.25}'),
  ('roof','flat_roof','mat.white_composite',0,'','{"y":2.1,"z":-0.15,"w":2.9,"d":2.5,"t":0.06,"overhang":0}'),
  ('parapet','wall','mat.white_composite',1,'','{"y":2.16,"z":-0.15,"w":2.8,"d":2.4,"h":0.12,"t":0.1,"ring":1}'),
  ('drum','cylinder','mat.smart_glass',0,'','{"y":2.16,"z":-0.15,"w":2,"h":0.3,"segments":16}'),
  ('drum_glow','cylinder','mat.light_strip',0,'','{"y":2.44,"z":-0.15,"w":2.04,"h":0.03,"segments":16,"far":0}'),
  ('dome','dome','mat.white_composite',0,'','{"y":2.46,"z":-0.15,"w":2.1,"h":1,"segments":16}'),
  ('oculus','cylinder','mat.smart_glass',0,'','{"y":3.38,"z":-0.15,"w":0.5,"h":0.12,"segments":8}'),
  ('oculus_cap','cylinder','mat.white_composite',1,'','{"y":3.5,"z":-0.15,"w":0.58,"h":0.06,"segments":8}'),
  ('lobby','box','mat.smart_glass',0,'','{"y":0.25,"z":1.33,"w":1.6,"d":0.56,"h":0.9}'),
  ('lobby_roof','flat_roof','mat.white_composite',0,'','{"y":1.15,"z":1.3,"w":1.8,"d":0.8,"t":0.08,"overhang":0}'),
  ('lobby_glow','box','mat.light_strip',0,'','{"y":1.12,"z":1.69,"w":1.78,"d":0.03,"h":0.03,"far":0}'),
  ('doors','box','mat.graphite',0,'','{"y":0.25,"z":1.62,"w":0.5,"d":0.02,"h":0.6,"far":0}'),
  ('lamps','cylinder','mat.white_composite',0,'','{"x":-1.55,"y":0.25,"z":1.55,"w":0.05,"h":0.9,"segments":4,"repeat":2,"dx":3.1,"far":0}'),
  ('lamp_heads','box','mat.light_strip',0,'','{"x":-1.55,"y":1.15,"z":1.55,"w":0.1,"d":0.1,"h":0.12,"repeat":2,"dx":3.1,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.library_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.library'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.library_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
