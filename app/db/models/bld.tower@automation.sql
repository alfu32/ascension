-- bld.tower@automation: Future tower (mesh.tower_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_automation','Future tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.composite_panel',0,'','{"w":1.75,"d":1.75,"h":0.45,"taper":0.1}'),
  ('base_glow','box','mat.light_strip',0,'','{"y":0.45,"w":1.42,"d":1.42,"h":0.03,"far":0}'),
  ('shaft','cylinder','mat.composite_panel',0,'','{"y":0.45,"w":1.15,"h":2.95,"taper":0.12,"segments":10}'),
  ('slits','cylinder','mat.smart_glass',0,'','{"y":1.5,"w":1.1,"h":0.16,"segments":10,"far":0}'),
  ('band','cylinder','mat.white_composite',1,'','{"y":2.8,"w":1.1,"h":0.6,"segments":10}'),
  ('cab_floor','box','mat.white_composite',1,'','{"y":3.4,"w":1.75,"d":1.75,"h":0.12}'),
  ('cab','box','mat.smart_glass',0,'','{"y":3.52,"w":1.5,"d":1.5,"h":0.45}'),
  ('cab_roof','box','mat.white_composite',0,'','{"y":3.97,"w":1.75,"d":1.75,"h":0.1}'),
  ('cab_glow','box','mat.light_strip',0,'','{"y":3.93,"w":1.77,"d":1.77,"h":0.03,"far":0}'),
  ('merlons','crenellation','mat.white_composite',1,'','{"y":4.07,"w":1.75,"d":1.75,"h":0.24,"t":0.12,"merlon":0.3,"ring":1}'),
  ('sensor','dome','mat.white_composite',1,'','{"x":-0.4,"y":4.07,"z":0.4,"w":0.5,"h":0.32,"segments":8}'),
  ('mast','cylinder','mat.white_composite',0,'','{"x":0.45,"y":4.07,"z":-0.45,"w":0.05,"h":1.3,"segments":4}'),
  ('mast_light','box','mat.light_strip',0,'','{"x":0.45,"y":5.37,"z":-0.45,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.45,"y":4.07,"z":0.45,"h":1.6,"fw":0.7,"fh":0.46,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.tower_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
