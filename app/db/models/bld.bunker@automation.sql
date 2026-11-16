-- bld.bunker@automation: Future bunker (mesh.bunker_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bunker_automation','Future bunker');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bunker_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('berm','plinth','mat.white_composite',0,'','{"w":1.85,"d":1.85,"h":0.2,"steps":2,"inset":0.1}'),
  ('body','cylinder','mat.composite_panel',0,'','{"y":0.2,"w":1.5,"h":0.32,"segments":10}'),
  ('slit','cylinder','mat.graphite',0,'','{"y":0.42,"w":1.52,"h":0.07,"segments":10}'),
  ('dome','dome','mat.white_composite',0,'','{"y":0.52,"w":1.5,"h":0.5,"segments":10}'),
  ('ring','torus','mat.white_composite',1,'','{"y":0.46,"w":1.66,"t":0.14,"segments":10,"sides":4}'),
  ('ring_glow','cylinder','mat.light_strip',0,'','{"y":0.24,"w":1.54,"h":0.03,"segments":10,"far":0}'),
  ('cupola','dome','mat.graphite',0,'','{"x":0.15,"y":0.95,"z":0.1,"w":0.4,"h":0.22,"segments":8}'),
  ('gun','cylinder','mat.graphite',0,'','{"x":0.15,"y":1.03,"z":0.2,"w":0.05,"h":0.4,"segments":4,"pitch":90}'),
  ('sensor','box','mat.light_strip',0,'','{"x":-0.3,"y":0.98,"z":-0.15,"w":0.08,"d":0.08,"h":0.06,"far":0}'),
  ('entrance','box','mat.white_composite',0,'','{"y":0.2,"z":-0.82,"yaw":180,"w":0.5,"d":0.3,"h":0.42,"door":0.26,"door_h":0.35,"door_colour":"#2c3a40"}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.45,"y":0.85,"z":0.15,"h":1.1,"fw":0.45,"fh":0.3,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bunker_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bunker'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.bunker_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
